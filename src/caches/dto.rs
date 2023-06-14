use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum PositionSide {
    Buy,
    Sell,
}

impl From<i32> for PositionSide {
    fn from(value: i32) -> Self {
        match value {
            0 => PositionSide::Buy,
            1 => PositionSide::Sell,
            _ => panic!("Invalid position side"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChargeSettlementEvent {
    pub position_id: String,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum ExecutionClosePositionReason {
    ClientCommand = 0,
    StopOut = 1,
    TakeProfit = 2,
    StopLoss = 3,
    ForceClose = 4,
}

#[derive(Debug, Clone)]
pub enum ExecutionPendingOrderType {
    BuyStop = 0,
    BuyLimit = 1,
    SellStop = 2,
    SellLimit = 3,
}
