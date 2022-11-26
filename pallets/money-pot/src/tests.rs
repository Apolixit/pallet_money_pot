use crate::{Pallet as MoneyPot, *};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::{ArithmeticError, DispatchError};

#[cfg(test)]
mod tests {
	use frame_support::pallet_prelude::DispatchResult;
	use crate::mock::{ExtBuilder, Origin, Test, Balances};

use super::*;

    pub type MockAccountId = u64;

	pub const ALICE: MockAccountId = 1;
	pub const BOB: MockAccountId = 2;
	pub const APO: MockAccountId = 3;

	#[test]
	pub fn create_moneypot_with_enought_balance_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			let origin_alice = Origin::signed(ALICE);
            log::info!("{:?}", MoneyPot::<Test>::pallet_constants_metadata());
            log::info!("Balances : Alice = {:?}", Balances::free_balance(ALICE));
            assert_eq!(10, Balances::free_balance(ALICE));
			assert_eq!(MoneyPot::<Test>::create_with_limit_amount(origin_alice, BOB, 1000), DispatchResult::Ok(()));
		});
	}

    #[test]
	pub fn create_moneypot_with_no_balance_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			let origin_apo = Origin::signed(APO);
            Balances::set_balance(origin_apo.clone(), APO, 0, 0);

			assert_eq!(0, Balances::free_balance(APO));

			assert_eq!(MoneyPot::<Test>::create_with_limit_amount(origin_apo.clone(), BOB, 1000), DispatchResult::Ok(()));
		});
	}

	#[test]
	pub fn contribute_moneypot_with_enought_fund_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			let origin_alice = Origin::signed(APO);
			MoneyPot::<Test>::create_with_limit_amount(origin_alice.clone(), BOB, 1000);

			MoneyPot::<Test>::add_balance(origin_alice, ref_hash, amount)
		});
	}

	#[test]
	pub fn contribute_moneypot_with_not_enought_fund_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			let origin_alice = Origin::signed(ALICE);

			let result = MoneyPot::<Test>::create_with_limit_amount(origin_alice.clone(), BOB, 1000);
			Balances::set_balance(origin_alice.clone(), ALICE, 10, 0);

			MoneyPot::<Test>::add_balance(origin_alice, result.unwrap(), 20)
		});
	}
}
