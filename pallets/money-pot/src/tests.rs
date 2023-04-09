use crate::{Pallet as MoneyPot, *};
use frame_support::{assert_noop};

#[cfg(test)]
mod tests {
	use frame_support::{pallet_prelude::DispatchResult};
	use crate::mock::{ExtBuilder, Origin, Test, Balances, MaxMoneyPotCurrentlyOpen, System, StepContribution, run_to_block, MinContribution};

use super::*;

    pub type MockAccountId = u64;

	pub const ALICE: MockAccountId = 1;
	pub const BOB: MockAccountId = 2;
	pub const APOLIXIT: MockAccountId = 3;

	#[test]
	pub fn default_money_pot_count() {
		ExtBuilder::default().build().execute_with(|| {
			assert_eq!(MoneyPotsCount::<Test>::get(), 0);
		});
	}

	#[test]
	pub fn create_moneypot_with_enought_balance_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			let balance_alice: u64 = 2000000000;
			let origin_alice = Origin::signed(ALICE);
			let _ = Balances::set_balance(Origin::root(), ALICE, balance_alice, 0);

            assert_eq!(balance_alice, Balances::free_balance(ALICE));
			
			assert_eq!(MoneyPot::<Test>::create_with_limit_amount(origin_alice, BOB, 1000), DispatchResult::Ok(()));
			
			// Now we got one
			assert_eq!(MoneyPotsCount::<Test>::get(), 1);
		});
	}

	#[test]
	pub fn create_more_moneypot_than_max_allowed_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			let balance_alice: u64 = 2000000000;
			Balances::set_balance(Origin::root(), ALICE, balance_alice, 0).unwrap();

			assert_eq!(MaxMoneyPotCurrentlyOpen::get(), 5);

			for n in 0 .. MaxMoneyPotCurrentlyOpen::get() {
				MoneyPot::<Test>::create_with_limit_amount(Origin::signed(ALICE), BOB, (1000 * (1 + n)).into()).unwrap();
				assert_eq!(MoneyPotsCount::<Test>::get(), n + 1);
			}
			
			assert_noop!(
				MoneyPot::<Test>::create_with_limit_amount(Origin::signed(ALICE), BOB, 1050), 
				Error::<Test>::MaxOpenOverflow);
			
		});
	}

	#[test]
	pub fn contribute_moneypot_with_enought_fund_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			let target_amount = 10_000u64;
			// Get current block
			let current_block = System::block_number(); 

			// Create a money pot
			MoneyPot::<Test>::create_with_limit_amount(Origin::signed(APOLIXIT), BOB, target_amount).unwrap();
			
			// Get its hash
			let ref_hash = MoneyPotsHash::<Test>::get(1).unwrap();
			let money_pot_opt = MoneyPots::<Test>::get(ref_hash);

			// Contribute
			MoneyPot::<Test>::add_balance(Origin::signed(APOLIXIT), ref_hash, 2000).unwrap();

			assert!(money_pot_opt.is_some());
			let money_pot = money_pot_opt.unwrap();

			assert_eq!(money_pot.is_active, true);
			assert_eq!(money_pot.owner, APOLIXIT);
			assert_eq!(money_pot.receiver, BOB);
			assert_eq!(money_pot.start_time, current_block);
			assert_eq!(money_pot.end_time, Some(EndType::AmountReached { amount_type: AmountType::Native, amount: target_amount }));
		});
	}

	#[test]
	pub fn contribute_moneypot_then_contribute_until_close_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			let target_amount = 10_000u64;

			// Get Apolixit and BOB balances at start
			let (initial_apolixit_free_balances, initial_apolixit_reserved_balances, apolixit_contribution) = (
				Balances::free_balance(APOLIXIT), 
				Balances::reserved_balance(APOLIXIT),
				2000u64);
			let (initial_alice_free_balances, initial_alice_reserved_balances, alice_contribution) = (
				Balances::free_balance(ALICE), 
				Balances::reserved_balance(ALICE),
				9000u64);

			let initial_bob_free_balances = Balances::free_balance(BOB);

			assert_eq!(0, initial_apolixit_reserved_balances);
			assert_eq!(0, initial_alice_reserved_balances);
			
			// Create a money pot
			MoneyPot::<Test>::create_with_limit_amount(Origin::signed(APOLIXIT), BOB, target_amount).unwrap();
			
			// Get money pot hash
			let ref_hash = MoneyPotsHash::<Test>::get(1).unwrap();

			// Contribute
			MoneyPot::<Test>::add_balance(Origin::signed(APOLIXIT), ref_hash, apolixit_contribution).unwrap();
			
			// Apolixit usable balance is now decrease by apolixit_contribution amount
			assert_eq!(Balances::usable_balance(APOLIXIT), initial_apolixit_free_balances - apolixit_contribution);

			// BOB free balance is always the same as start because no funds has been transfered
			assert_eq!(Balances::free_balance(BOB), initial_bob_free_balances);

			MoneyPot::<Test>::add_balance(Origin::signed(ALICE), ref_hash, alice_contribution).unwrap();
			assert_eq!(Balances::usable_balance(ALICE), initial_alice_free_balances - alice_contribution);
			
			let money_pot = MoneyPots::<Test>::get(ref_hash).unwrap();
			//Now money pot should be close because we reached target amount
			assert_eq!(money_pot.is_active, false);

			// Apolixit free balance is now decrease by apolixit_contribution amount
			assert_eq!(Balances::free_balance(APOLIXIT), initial_apolixit_free_balances - apolixit_contribution);
			// Same for Alice
			assert_eq!(Balances::free_balance(ALICE), initial_alice_free_balances - alice_contribution);

			// Check BOB got his money
			assert_eq!(Balances::free_balance(BOB), initial_bob_free_balances + apolixit_contribution + alice_contribution);

			// If we try to contribute to a close money pot, we got an error
			assert_noop!(MoneyPot::<Test>::add_balance(Origin::signed(APOLIXIT), ref_hash, 4000), Error::<Test>::MoneyPotIsClose);
		});
	}

	#[test]
	pub fn contribute_moneypot_with_fund_lower_than_min_contribution_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			let target_amount = 10_000u64;
			let min_contribution = mock::MinContribution::get();
			let step_contribution = StepContribution::get();

			// Create a money pot
			MoneyPot::<Test>::create_with_limit_amount(Origin::signed(APOLIXIT), BOB, target_amount).unwrap();
			
			// Get its hash
			let ref_hash = MoneyPotsHash::<Test>::get(1).unwrap();

			// Contribute
			assert_noop!(
				MoneyPot::<Test>::add_balance(Origin::signed(APOLIXIT), ref_hash, (min_contribution - step_contribution).into()), 
				Error::<Test>::AmountToLow);
		});
	}

	#[test]
	pub fn contribute_moneypot_with_balance_does_not_respect_contribution_step_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			let target_amount = 10_000u64;
			let step_contribution = StepContribution::get();
			// Create a money pot
			MoneyPot::<Test>::create_with_limit_amount(Origin::signed(APOLIXIT), BOB, target_amount).unwrap();
			
			// Get its hash
			let ref_hash = MoneyPotsHash::<Test>::get(1).unwrap();

			// Contribute
			assert_noop!(
				MoneyPot::<Test>::add_balance(Origin::signed(APOLIXIT), ref_hash, (step_contribution - 1).into()), 
				Error::<Test>::InvalidAmountStep);
		});
	}

	#[test]
	pub fn contribute_moneypot_with_not_enought_balance_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			let target_amount = 40_000u64;
			Balances::set_balance(Origin::root(), ALICE, 20_000u64, 0).unwrap();

			// Create a money pot
			MoneyPot::<Test>::create_with_limit_amount(Origin::signed(ALICE), BOB, target_amount).unwrap();
			
			// Get its hash
			let ref_hash = MoneyPotsHash::<Test>::get(1).unwrap();

			// Contribute
			assert_noop!(
				MoneyPot::<Test>::add_balance(Origin::signed(ALICE), ref_hash, 30_000u64), 
				Error::<Test>::NotEnoughBalance);
		});
	}

	#[test]
	pub fn create_moneypot_with_endtime_block_should_work() {
		ExtBuilder::default().build().execute_with(|| {
			let current_block = System::block_number();
			let target_block = current_block + 10;

			let apolixit_contribution = 2000u64;
			let alice_contribution = 2000u64;

			let initial_bob_free_balances = Balances::free_balance(BOB);

			// Create a money pot
			MoneyPot::<Test>::create_with_limit_block(Origin::signed(ALICE), BOB, target_block).unwrap();
			
			// Get its hash
			let ref_hash = MoneyPotsHash::<Test>::get(1).unwrap();

			// Contribute
			MoneyPot::<Test>::add_balance(Origin::signed(APOLIXIT), ref_hash, apolixit_contribution).unwrap();
			MoneyPot::<Test>::add_balance(Origin::signed(ALICE), ref_hash, alice_contribution).unwrap();

			assert_eq!(MoneyPots::<Test>::get(ref_hash).unwrap().is_active, true);

			// BOB didn't received his money yet
			assert_eq!(Balances::free_balance(BOB), initial_bob_free_balances);

			run_to_block(target_block);
			assert_eq!(Balances::free_balance(BOB), initial_bob_free_balances + apolixit_contribution + alice_contribution);

			assert_eq!(MoneyPots::<Test>::get(ref_hash).unwrap().is_active, false);
		});
	}

	#[test]
	pub fn create_moneypot_with_end_time_block_invalid_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			run_to_block(System::block_number() + 20);

			assert_noop!(
				MoneyPot::<Test>::create_with_limit_block(Origin::signed(ALICE), BOB, System::block_number() - 10),
				Error::<Test>::ScheduleError
			);
		});
	}

	#[test]
	pub fn create_moneypot_with_zero_balance_target_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			assert_noop!(
				MoneyPot::<Test>::create_with_limit_amount(Origin::signed(APOLIXIT), BOB, 0),
			Error::<Test>::AmountToLow);
		});
	}

	#[test]
	pub fn create_moneypot_with_zero_balance_target_inf_minimum_contribution_should_fail() {
		ExtBuilder::default().build().execute_with(|| {
			assert_noop!(
				MoneyPot::<Test>::create_with_limit_amount(Origin::signed(APOLIXIT), BOB, (MinContribution::get() - StepContribution::get()).into()),
			Error::<Test>::AmountToLow);
		});
	}
}
