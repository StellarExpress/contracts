use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAuthorized = 3,
    ShipmentNotFound = 4,
    InvalidAmount = 5,
    InvalidReleaseSplit = 6,
    ShipmentNotOpen = 7,
    ShipmentAlreadyAccepted = 8,
    WrongCarrier = 9,
    WrongReceiver = 10,
    InvalidStatusForAction = 11,
    NotExpiredYet = 12,
    InvalidDisputeSplit = 13,
    NotDisputed = 14,
}
