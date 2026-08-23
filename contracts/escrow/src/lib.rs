#![no_std]

mod errors;
mod types;

#[cfg(test)]
mod test;

use errors::Error;
use soroban_sdk::{contract, contractimpl, token, Address, Env, Vec};
use types::{DataKey, Shipment, ShipmentCategory, ShipmentStatus};

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    // ---------------------------------------------------------------
    // Setup
    // ---------------------------------------------------------------

    /// One-time setup. `arbiter` is the address trusted to resolve disputes
    /// across every shipment this contract instance manages.
    pub fn init(env: Env, admin: Address, arbiter: Address) -> Result<(), Error> {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Arbiter, &arbiter);
        Ok(())
    }

    pub fn get_arbiter(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Arbiter)
            .ok_or(Error::NotInitialized)
    }

    // ---------------------------------------------------------------
    // Shipment lifecycle
    // ---------------------------------------------------------------

    /// Funds an escrow for a new shipment. The sender transfers `total_amount`
    /// into the contract immediately; it is released in two installments —
    /// `pickup_release_bps` of it when a carrier confirms pickup, the rest
    /// when the receiver confirms delivery. The shipment starts `Open` for
    /// any carrier to accept.
    pub fn create_shipment(
        env: Env,
        sender: Address,
        receiver: Address,
        asset: Address,
        total_amount: i128,
        pickup_release_bps: u32,
        delivery_deadline_ledger: u32,
        category: ShipmentCategory,
    ) -> Result<u64, Error> {
        sender.require_auth();
        if total_amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if pickup_release_bps > 10_000 {
            return Err(Error::InvalidReleaseSplit);
        }

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&sender, &env.current_contract_address(), &total_amount);

        let id = next_id(&env);
        let shipment = Shipment {
            id,
            sender: sender.clone(),
            carrier: None,
            receiver,
            asset,
            total_amount,
            released_amount: 0,
            pickup_release_bps,
            category,
            status: ShipmentStatus::Open,
            delivery_deadline_ledger,
            created_at: env.ledger().timestamp(),
        };
        save_shipment(&env, &shipment);

        append_id(&env, &DataKey::SenderShipments(sender), id);
        append_id(&env, &DataKey::OpenShipments, id);
        Ok(id)
    }

    /// A carrier claims an open shipment.
    pub fn accept_shipment(env: Env, shipment_id: u64, carrier: Address) -> Result<(), Error> {
        carrier.require_auth();
        let mut shipment = load_shipment(&env, shipment_id)?;
        if !matches!(shipment.status, ShipmentStatus::Open) {
            return Err(Error::ShipmentAlreadyAccepted);
        }
        shipment.carrier = Some(carrier.clone());
        shipment.status = ShipmentStatus::Accepted;
        save_shipment(&env, &shipment);

        remove_id(&env, &DataKey::OpenShipments, shipment_id);
        append_id(&env, &DataKey::CarrierShipments(carrier), shipment_id);
        Ok(())
    }

    /// The carrier confirms they've physically picked up the goods. Releases
    /// `pickup_release_bps` of the total to the carrier immediately.
    pub fn confirm_pickup(env: Env, shipment_id: u64, carrier: Address) -> Result<(), Error> {
        carrier.require_auth();
        let mut shipment = load_shipment(&env, shipment_id)?;
        require_carrier(&shipment, &carrier)?;
        if !matches!(shipment.status, ShipmentStatus::Accepted) {
            return Err(Error::InvalidStatusForAction);
        }

        let pickup_amount = (shipment.total_amount * shipment.pickup_release_bps as i128) / 10_000;
        if pickup_amount > 0 {
            transfer_out(&env, &shipment.asset, &carrier, pickup_amount);
        }
        shipment.released_amount += pickup_amount;
        shipment.status = ShipmentStatus::InTransit;
        save_shipment(&env, &shipment);
        Ok(())
    }

    /// The receiver confirms the goods arrived. Releases the remaining
    /// escrow balance to the carrier.
    pub fn confirm_delivery(env: Env, shipment_id: u64, receiver: Address) -> Result<(), Error> {
        receiver.require_auth();
        let mut shipment = load_shipment(&env, shipment_id)?;
        if receiver != shipment.receiver {
            return Err(Error::WrongReceiver);
        }
        if !matches!(shipment.status, ShipmentStatus::InTransit) {
            return Err(Error::InvalidStatusForAction);
        }

        let remaining = shipment.total_amount - shipment.released_amount;
        let carrier = shipment.carrier.clone().ok_or(Error::ShipmentNotOpen)?;
        if remaining > 0 {
            transfer_out(&env, &shipment.asset, &carrier, remaining);
        }
        shipment.released_amount = shipment.total_amount;
        shipment.status = ShipmentStatus::Delivered;
        save_shipment(&env, &shipment);
        Ok(())
    }

    /// The sender cancels before any carrier has accepted the job; full
    /// refund.
    pub fn cancel_shipment(env: Env, shipment_id: u64, sender: Address) -> Result<(), Error> {
        sender.require_auth();
        let mut shipment = load_shipment(&env, shipment_id)?;
        if sender != shipment.sender {
            return Err(Error::NotAuthorized);
        }
        if !matches!(shipment.status, ShipmentStatus::Open) {
            return Err(Error::InvalidStatusForAction);
        }

        let remaining = shipment.total_amount - shipment.released_amount;
        if remaining > 0 {
            transfer_out(&env, &shipment.asset, &sender, remaining);
        }
        shipment.released_amount = shipment.total_amount;
        shipment.status = ShipmentStatus::Cancelled;
        save_shipment(&env, &shipment);
        remove_id(&env, &DataKey::OpenShipments, shipment_id);
        Ok(())
    }

    /// If no carrier ever accepted the shipment and the deadline has passed,
    /// the sender can reclaim the full escrow.
    pub fn reclaim_expired(env: Env, shipment_id: u64, sender: Address) -> Result<(), Error> {
        sender.require_auth();
        let mut shipment = load_shipment(&env, shipment_id)?;
        if sender != shipment.sender {
            return Err(Error::NotAuthorized);
        }
        if !matches!(shipment.status, ShipmentStatus::Open) {
            return Err(Error::InvalidStatusForAction);
        }
        if env.ledger().sequence() < shipment.delivery_deadline_ledger {
            return Err(Error::NotExpiredYet);
        }

        let remaining = shipment.total_amount - shipment.released_amount;
        if remaining > 0 {
            transfer_out(&env, &shipment.asset, &sender, remaining);
        }
        shipment.released_amount = shipment.total_amount;
        shipment.status = ShipmentStatus::Expired;
        save_shipment(&env, &shipment);
        remove_id(&env, &DataKey::OpenShipments, shipment_id);
        Ok(())
    }

    // ---------------------------------------------------------------
    // Disputes
    // ---------------------------------------------------------------

    /// Any party to the shipment (sender, carrier, or receiver) can flag it
    /// as disputed once it's in transit, freezing further releases until
    /// the arbiter resolves it.
    pub fn raise_dispute(env: Env, shipment_id: u64, caller: Address) -> Result<(), Error> {
        caller.require_auth();
        let mut shipment = load_shipment(&env, shipment_id)?;
        let is_party = caller == shipment.sender
            || caller == shipment.receiver
            || shipment.carrier.as_ref() == Some(&caller);
        if !is_party {
            return Err(Error::NotAuthorized);
        }
        if !matches!(
            shipment.status,
            ShipmentStatus::Accepted | ShipmentStatus::InTransit
        ) {
            return Err(Error::InvalidStatusForAction);
        }
        shipment.status = ShipmentStatus::Disputed;
        save_shipment(&env, &shipment);
        Ok(())
    }

    /// The arbiter splits whatever remains in escrow between the sender and
    /// the carrier. `sender_bps` is the sender's share in basis points; the
    /// carrier gets the rest.
    pub fn resolve_dispute(
        env: Env,
        shipment_id: u64,
        arbiter: Address,
        sender_bps: u32,
    ) -> Result<(), Error> {
        arbiter.require_auth();
        let configured_arbiter: Address = env
            .storage()
            .instance()
            .get(&DataKey::Arbiter)
            .ok_or(Error::NotInitialized)?;
        if arbiter != configured_arbiter {
            return Err(Error::NotAuthorized);
        }
        let mut shipment = load_shipment(&env, shipment_id)?;
        if !matches!(shipment.status, ShipmentStatus::Disputed) {
            return Err(Error::NotDisputed);
        }
        if sender_bps > 10_000 {
            return Err(Error::InvalidDisputeSplit);
        }

        let remaining = shipment.total_amount - shipment.released_amount;
        let sender_share = (remaining * sender_bps as i128) / 10_000;
        let carrier_share = remaining - sender_share;

        if sender_share > 0 {
            transfer_out(&env, &shipment.asset, &shipment.sender, sender_share);
        }
        if carrier_share > 0 {
            let carrier = shipment.carrier.clone().ok_or(Error::ShipmentNotOpen)?;
            transfer_out(&env, &shipment.asset, &carrier, carrier_share);
        }
        shipment.released_amount = shipment.total_amount;
        shipment.status = ShipmentStatus::Resolved;
        save_shipment(&env, &shipment);
        Ok(())
    }

    // ---------------------------------------------------------------
    // Read-only views
    // ---------------------------------------------------------------

    pub fn get_shipment(env: Env, shipment_id: u64) -> Result<Shipment, Error> {
        load_shipment(&env, shipment_id)
    }

    pub fn list_shipments_by_sender(env: Env, sender: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::SenderShipments(sender))
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn list_shipments_by_carrier(env: Env, carrier: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::CarrierShipments(carrier))
            .unwrap_or_else(|| Vec::new(&env))
    }

    pub fn list_open_shipments(env: Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::OpenShipments)
            .unwrap_or_else(|| Vec::new(&env))
    }
}

// ---------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------

fn next_id(env: &Env) -> u64 {
    let current: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextShipmentId)
        .unwrap_or(0);
    let next = current + 1;
    env.storage()
        .instance()
        .set(&DataKey::NextShipmentId, &next);
    next
}

fn load_shipment(env: &Env, shipment_id: u64) -> Result<Shipment, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Shipment(shipment_id))
        .ok_or(Error::ShipmentNotFound)
}

fn save_shipment(env: &Env, shipment: &Shipment) {
    env.storage()
        .persistent()
        .set(&DataKey::Shipment(shipment.id), shipment);
}

fn require_carrier(shipment: &Shipment, carrier: &Address) -> Result<(), Error> {
    match &shipment.carrier {
        Some(c) if c == carrier => Ok(()),
        Some(_) => Err(Error::WrongCarrier),
        None => Err(Error::ShipmentNotOpen),
    }
}

fn transfer_out(env: &Env, asset: &Address, to: &Address, amount: i128) {
    let token_client = token::Client::new(env, asset);
    token_client.transfer(&env.current_contract_address(), to, &amount);
}

fn append_id(env: &Env, key: &DataKey, id: u64) {
    let mut list: Vec<u64> = env
        .storage()
        .persistent()
        .get(key)
        .unwrap_or_else(|| Vec::new(env));
    list.push_back(id);
    env.storage().persistent().set(key, &list);
}

fn remove_id(env: &Env, key: &DataKey, id: u64) {
    let list: Vec<u64> = env
        .storage()
        .persistent()
        .get(key)
        .unwrap_or_else(|| Vec::new(env));
    let mut updated = Vec::new(env);
    for existing in list.iter() {
        if existing != id {
            updated.push_back(existing);
        }
    }
    env.storage().persistent().set(key, &updated);
}
