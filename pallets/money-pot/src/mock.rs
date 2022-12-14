use frame_support::parameter_types;

use super::*;
use crate::{self as pallet_money_pot};


frame_support::construct_runtime!(
	pub enum Test
	where
		Block = MockBloc<Test>,
		NodeBlock = MockBloc<Test>,
		UncheckedExtrinsic = MockUncheckedExtrinsic<Test>,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		MoneyPot: pallet_money_pot::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	/// The identifier used to distinguish between accounts.
	type AccountId = u32;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = IdentityLookup<Self::AccountId>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = u64;
	/// The index type for blocks.
	type BlockNumber = u64;
	/// The type for hashing blocks and tries.
	type Hash = H256;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = Header;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = ConstU64<250>;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = ();
	/// Version of the runtime.
	type Version = ();
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<u64>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = ConstU16<42>;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const MaxMoneyPotCurrentlyOpen: u32 = 5;
	pub const MaxMoneyPotContributors: u32 = 1000;
	pub const MinContribution: u32 = 5;
	pub const StepContribution: u32 = 5;
}

impl pallet_money_pot::Config for Test {
	type Event = Event;
	type Currency = u64;
	type Scheduler = Scheduler;
	type MaxMoneyPotCurrentlyOpen = MaxMoneyPotCurrentlyOpen;
	type MaxMoneyPotContributors = MaxMoneyPotContributors;
	type MinContribution = MinContribution;
	type StepContribution = StepContribution;
	type ExistentialDeposit = ConstU128<MOCK_EXISTENTIAL_DEPOSIT>;
	type PalletsOrigin = OriginCaller;
	type Schedulable = Call;
	/// Max one year
	type MaxBlockNumberEndTime = ConstU32<1_000_000>;
}

/// Existential deposit.
pub const MOCK_EXISTENTIAL_DEPOSIT: u128 = 1;

impl pallet_balances::Config for Test {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = u64;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<MOCK_EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = ();
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(70) * BlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Test {
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = ();
    type ScheduleOrigin = EnsureRoot<u64>; //EitherOfDiverse<EnsureRoot<u64>, EnsureSignedBy<One, u64>>;
    type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
    type MaxScheduledPerBlock = ConstU32<50>;
    type WeightInfo = ();
    type PreimageProvider = ();
    type NoPreimagePostponement = ();
}

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let init = system::GenesisConfig::default().build_storage::<Test>().unwrap();
		pallet_balances::GenesisConfig::<Test> {
			balances: vec![((1, 10), (2, 20), (3, 30), (4, 40), (5, 50), (6, 60))]
		}
		.assimilate_storage(&mut init)
		.unwrap();

		let mut exp = sp_io::TestExternalities::new(init);
		ext.execute_with(|| System::set_block_number(1));

		ext
	}
}

pub fn run_to_block(n: u64) {
	let current_block = System::block_number();
	while current_block < n {
		Scheduler::on_finalize(current_block);
		Scheduler::set_block_number(current_block + 1);
		Scheduler::on_initialize(System::block_number());
	}
}