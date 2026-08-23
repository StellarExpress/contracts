#![cfg(test)]

use crate::types::ShipmentCategory;
use crate::{errors::Error, EscrowContract, EscrowContractClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{token, Address, Env};

fn setup() -> (Env, EscrowContractClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let asset_admin = Address::generate(&env);
    let sac_id = env.register_stellar_asset_contract_v2(asset_admin);
    let asset = sac_id.address();

    let admin = Address::generate(&env);
    let arbiter = Address::generate(&env);
    client.init(&admin, &arbiter);

    (env, client, asset, arbiter)
}

fn mint(env: &Env, asset: &Address, to: &Address, amount: i128) {
    token::StellarAssetClient::new(env, asset).mint(to, &amount);
}

fn balance_of(env: &Env, asset: &Address, who: &Address) -> i128 {
    token::Client::new(env, asset).balance(who)
}

#[test]
fn init_rejects_double_initialization() {
    let (_env, client, _asset, _arbiter) = setup();
    let admin2 = Address::generate(&_env);
    let arbiter2 = Address::generate(&_env);
    let result = client.try_init(&admin2, &arbiter2);
    assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn create_shipment_funds_escrow() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    mint(&env, &asset, &sender, 1_000);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &1_000,
        &5_000,
        &1_000,
        &ShipmentCategory::General,
    );

    assert_eq!(balance_of(&env, &asset, &sender), 0);
    let shipment = client.get_shipment(&id);
    assert_eq!(shipment.total_amount, 1_000);
    assert_eq!(shipment.released_amount, 0);
}

#[test]
fn accept_shipment_assigns_carrier_and_leaves_open_list() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let carrier = Address::generate(&env);
    mint(&env, &asset, &sender, 500);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &500,
        &5_000,
        &1_000,
        &ShipmentCategory::Food,
    );
    assert_eq!(client.list_open_shipments().len(), 1);

    client.accept_shipment(&id, &carrier);

    assert_eq!(client.list_open_shipments().len(), 0);
    assert_eq!(client.list_shipments_by_carrier(&carrier).len(), 1);
}

#[test]
fn confirm_pickup_releases_pickup_share_to_carrier() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let carrier = Address::generate(&env);
    mint(&env, &asset, &sender, 1_000);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &1_000,
        &4_000,
        &1_000,
        &ShipmentCategory::Electronics,
    );
    client.accept_shipment(&id, &carrier);
    client.confirm_pickup(&id, &carrier);

    assert_eq!(balance_of(&env, &asset, &carrier), 400);
    let shipment = client.get_shipment(&id);
    assert_eq!(shipment.released_amount, 400);
}

#[test]
fn confirm_delivery_releases_remaining_balance() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let carrier = Address::generate(&env);
    mint(&env, &asset, &sender, 1_000);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &1_000,
        &5_000,
        &1_000,
        &ShipmentCategory::General,
    );
    client.accept_shipment(&id, &carrier);
    client.confirm_pickup(&id, &carrier);
    client.confirm_delivery(&id, &receiver);

    assert_eq!(balance_of(&env, &asset, &carrier), 1_000);
    let shipment = client.get_shipment(&id);
    assert_eq!(shipment.released_amount, 1_000);
}

#[test]
fn wrong_carrier_cannot_confirm_pickup() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let carrier = Address::generate(&env);
    let impostor = Address::generate(&env);
    mint(&env, &asset, &sender, 500);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &500,
        &5_000,
        &1_000,
        &ShipmentCategory::General,
    );
    client.accept_shipment(&id, &carrier);

    let result = client.try_confirm_pickup(&id, &impostor);
    assert_eq!(result, Err(Ok(Error::WrongCarrier)));
}

#[test]
fn wrong_receiver_cannot_confirm_delivery() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let carrier = Address::generate(&env);
    let impostor = Address::generate(&env);
    mint(&env, &asset, &sender, 500);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &500,
        &5_000,
        &1_000,
        &ShipmentCategory::General,
    );
    client.accept_shipment(&id, &carrier);
    client.confirm_pickup(&id, &carrier);

    let result = client.try_confirm_delivery(&id, &impostor);
    assert_eq!(result, Err(Ok(Error::WrongReceiver)));
}

#[test]
fn cancel_shipment_refunds_sender_while_still_open() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    mint(&env, &asset, &sender, 750);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &750,
        &5_000,
        &1_000,
        &ShipmentCategory::Documents,
    );
    client.cancel_shipment(&id, &sender);

    assert_eq!(balance_of(&env, &asset, &sender), 750);
    assert_eq!(client.list_open_shipments().len(), 0);
}

#[test]
fn cannot_cancel_after_a_carrier_has_accepted() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let carrier = Address::generate(&env);
    mint(&env, &asset, &sender, 500);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &500,
        &5_000,
        &1_000,
        &ShipmentCategory::General,
    );
    client.accept_shipment(&id, &carrier);

    let result = client.try_cancel_shipment(&id, &sender);
    assert_eq!(result, Err(Ok(Error::InvalidStatusForAction)));
}

#[test]
fn reclaim_expired_refunds_sender_after_deadline() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    mint(&env, &asset, &sender, 600);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &600,
        &5_000,
        &100,
        &ShipmentCategory::Fragile,
    );

    let too_early = client.try_reclaim_expired(&id, &sender);
    assert_eq!(too_early, Err(Ok(Error::NotExpiredYet)));

    env.ledger().with_mut(|l| l.sequence_number += 200);
    client.reclaim_expired(&id, &sender);

    assert_eq!(balance_of(&env, &asset, &sender), 600);
}

#[test]
fn raise_dispute_then_resolve_splits_remaining_funds() {
    let (env, client, asset, arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let carrier = Address::generate(&env);
    mint(&env, &asset, &sender, 1_000);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &1_000,
        &5_000,
        &1_000,
        &ShipmentCategory::General,
    );
    client.accept_shipment(&id, &carrier);
    client.confirm_pickup(&id, &carrier);
    // 500 released to carrier already; 500 remains in escrow.

    client.raise_dispute(&id, &sender);
    client.resolve_dispute(&id, &arbiter, &7_000);

    // Sender gets 70% of the remaining 500 = 350, carrier gets the other 150
    // on top of the 500 already paid at pickup.
    assert_eq!(balance_of(&env, &asset, &sender), 350);
    assert_eq!(balance_of(&env, &asset, &carrier), 650);
}

#[test]
fn non_arbiter_cannot_resolve_dispute() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let carrier = Address::generate(&env);
    let impostor = Address::generate(&env);
    mint(&env, &asset, &sender, 500);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &500,
        &5_000,
        &1_000,
        &ShipmentCategory::General,
    );
    client.accept_shipment(&id, &carrier);
    client.raise_dispute(&id, &sender);

    let result = client.try_resolve_dispute(&id, &impostor, &5_000);
    assert_eq!(result, Err(Ok(Error::NotAuthorized)));
}

#[test]
fn invalid_pickup_release_split_rejected() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    mint(&env, &asset, &sender, 500);

    let result = client.try_create_shipment(
        &sender,
        &receiver,
        &asset,
        &500,
        &10_001,
        &1_000,
        &ShipmentCategory::General,
    );
    assert_eq!(result, Err(Ok(Error::InvalidReleaseSplit)));
}

#[test]
fn cannot_double_accept_a_shipment() {
    let (env, client, asset, _arbiter) = setup();
    let sender = Address::generate(&env);
    let receiver = Address::generate(&env);
    let carrier = Address::generate(&env);
    let other_carrier = Address::generate(&env);
    mint(&env, &asset, &sender, 500);

    let id = client.create_shipment(
        &sender,
        &receiver,
        &asset,
        &500,
        &5_000,
        &1_000,
        &ShipmentCategory::General,
    );
    client.accept_shipment(&id, &carrier);

    let result = client.try_accept_shipment(&id, &other_carrier);
    assert_eq!(result, Err(Ok(Error::ShipmentAlreadyAccepted)));
}
