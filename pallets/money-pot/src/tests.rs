// use crate::{Pallet as MoneyPot, *};
// use frame_support::{assert_noop, assert_ok};
// use sp_runtime::{ArithmeticError, DispatchError};

// #[cfg(test)]
// mod tests {
// 	use frame_system::Origin;
// 	use super::*;

//     pub type MockAccountId = u32;

// 	pub const ALICE: MockAccountId = 1;
// 	pub const BOB: MockAccountId = 2;
// 	pub const APO: MockAccountId = 3;

// 	#[test]
// 	pub fn create_moneupot_should_suceed() {
// 		ExtBuilder::default().build().execute_with(|| {
// 			let origin_alice = Origin::Signed(ALICE);
// 			MoneyPot::create_with_limit_amount(origin, receiver, amount)
// 		});
// 	}
// }
