use crate::{LidoStakeRouter, OneWayPair, ReserveStakeRouter};

pub type LidoReserveStakeAmm = OneWayPair<LidoStakeRouter, ReserveStakeRouter>;
