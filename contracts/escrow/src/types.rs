// Types type definitions.
use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ShipmentCategory {
    General,
    Food,
    Fragile,
    Electronics,
    Documents,
    Furniture,
}

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ShipmentStatus {
    /// Funded by the sender, waiting for a carrier to accept it.
    Open,
    /// A carrier has accepted the job but has not yet picked up the goods.
    Accepted,
    /// Carrier confirmed pickup; the pickup-release share has been paid out.
    InTransit,
    /// Receiver confirmed delivery; the remaining balance has been paid out.
    Delivered,
    /// Cancelled by the sender before a carrier accepted it; funds refunded.
    Cancelled,
    /// Reclaimed by the sender after the deadline passed with no carrier.
    Expired,
    /// Under dispute; awaiting arbiter resolution.
    Disputed,
    /// Dispute resolved; remaining balance split per the arbiter's decision.
    Resolved,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Shipment {
    pub id: u64,
    pub sender: Address,
    pub carrier: Option<Address>,
    pub receiver: Address,
    pub asset: Address,
    pub total_amount: i128,
    pub released_amount: i128,
    pub pickup_release_bps: u32,
    pub category: ShipmentCategory,
    pub status: ShipmentStatus,
    pub delivery_deadline_ledger: u32,
    pub created_at: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    Arbiter,
    NextShipmentId,
    Shipment(u64),
    SenderShipments(Address),
    CarrierShipments(Address),
    OpenShipments,
}