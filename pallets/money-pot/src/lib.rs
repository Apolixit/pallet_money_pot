#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

// sp-std = { version = "4.0.0", default-features = false, git = "https://github.com/paritytech/substrate.git", branch = "polkadot-v0.9.28" }
// ./target/release/node-money-pot --dev
// ./target/release/node-money-pot purge-chain --dev

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{
		dispatch::Dispatchable,
		pallet_prelude::{DispatchResult, *},
		sp_runtime::traits::Hash,
		sp_runtime::SaturatedConversion,
		traits::{
			schedule::{DispatchTime, Named as ScheduleNamed},
			Currency, IsType, LockIdentifier, LockableCurrency,
		},
		BoundedVec, Twox64Concat,
	};
	use frame_system::{
		ensure_signed,
		pallet_prelude::{OriginFor, *},
	};
	use scale_info::TypeInfo;

	/// Classic incremental integer index
	type MoneyPotIndex = u32;
	/// Shortcut to access account
	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	/// Shortcut to access balance
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	// pub type CallOf<T> = <T as frame_system::Config>::RuntimeCall;

	/*
	 * Coming soon :
		*	Add a vault where every funds will be transfered to instead of lock amount in every account (which seems less secure)
	 */

	const ID_LOCK_MONEY_POT: LockIdentifier = *b"moneypot";

	#[derive(RuntimeDebug, Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct MoneyPot<T: Config> {
		/// Creator
		pub owner: AccountOf<T>,
		/// Person who will receive fund
		pub receiver: AccountOf<T>,
		/// When the pot will start
		pub start_time: T::BlockNumber,
		/// When the pot will end and funds transfer to receiver
		pub end_time: Option<EndType<T>>,
		/// Is currently active or not
		pub is_active: bool,
	}

	impl<T: Config> MoneyPot<T> {
		/// Create a new money pot with default value
		fn create(owner: &T::AccountId, receiver: &T::AccountId) -> MoneyPot<T> {
			log::info!("💰 [Money pot] - Create called");

			MoneyPot::<T> {
				owner: owner.clone(),
				receiver: receiver.clone(),
				start_time: <frame_system::Pallet<T>>::block_number(),
				end_time: None,
				is_active: true,
			}
		}

		/// Set end time to amount limit
		pub fn with_native_currency_limit(&mut self, amount: BalanceOf<T>) {
			self.end_time =
				Some(EndType::AmountReached { amount_type: AmountType::Native, amount });
		}

		/// Set end time to blocknumber limit
		pub fn with_end_block(&mut self, end_time: T::BlockNumber) {
			self.end_time = Some(EndType::Time(end_time));
		}

		/// Close the money pot
		pub fn close(&mut self) {
			self.is_active = false;
		}
	}

	/// Describe how the money pot will end
	#[derive(RuntimeDebug, Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub enum EndType<T: Config> {
		/// Finished on a specific block
		Time(T::BlockNumber),

		/// The fixed amount has been reached
		AmountReached { amount_type: AmountType, amount: BalanceOf<T> },
	}

	/// Currently not use, maybe soon...
	#[derive(RuntimeDebug, Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq)]
	pub enum AmountType {
		Native,
		DOT,
		USD,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		// Currency type for this pallet
		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;

		// Scheduler use to dispatch money pot when ending
		type Schedulable: Parameter + Dispatchable<Origin = Self::Origin> + From<Call<Self>>;
		type Scheduler: ScheduleNamed<Self::BlockNumber, Self::Schedulable, Self::PalletsOrigin>;

		/// The maximum money pot can be currently open by an account
		#[pallet::constant]
		type MaxMoneyPotCurrentlyOpen: Get<u32>;

		/// The maximum contributors for each money pot
		#[pallet::constant]
		type MaxMoneyPotContributors: Get<u32>;

		/// The minimum contribution amount to participate to a money pot
		#[pallet::constant]
		type MinContribution: Get<BalanceOf<Self>>;

		/// The contribution step
		#[pallet::constant]
		type StepContribution: Get<BalanceOf<Self>>;

		/// The minimum amount required to keep an account open.
		#[pallet::constant]
		type ExistentialDeposit: Get<BalanceOf<Self>>;

		/// The max block number (relative to current block number) to schedule a EndType::Time money pot
		#[pallet::constant]
		type MaxBlockNumberEndTime: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Money pot has been created
		Created { ref_hash: T::Hash },
		/// The money has been transfered to the receiver
		Transfered { ref_hash: T::Hash },
		/// Money has been added
		MoneyAdded { ref_hash: T::Hash, who: T::AccountId, amount: BalanceOf<T> },
		/// Money pot has been closed
		Closed { ref_hash: T::Hash },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The account has more than 'MaxMoneyPotCurrentlyOpen' money pot
		MaxOpenOverflow,
		/// Unstable state with a money pot which have no end time
		NoEndTimeSpecified,
		/// No one added money
		HasNoMoney,
		/// Money pot lifetime exceed 'MaxMoneyPotLifetime'
		LifetimeOverflow,
		/// Money pot lifetime is in the past
		LifetimeIsTooLow,
		/// The money pot does not exists
		DoesNotExists,
		/// The money pot already exists
		AlreadyExists,
		/// Contribution is too high
		NotEnoughBalance,
		/// The number of contributor is too high
		MaxMoneyPotContributors,
		/// Transfer failed for this account
		TransferFailed,
		/// Incompatible amount set with step constant defined
		InvalidAmountStep,
		/// The amount participation is too low
		AmountToLow,
		/// Someone tries to contribute to an unactive pool
		MoneyPotIsClose,
		/// Schedule error
		ScheduleError,
	}

	// Storage

	// Store the number of money pot created from the begining
	#[pallet::storage]
	#[pallet::getter(fn money_pot_count)]
	pub type MoneyPotsCount<T> = StorageValue<_, MoneyPotIndex, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn money_pots_hash)]
	pub type MoneyPotsHash<T: Config> = StorageMap<_, Twox64Concat, MoneyPotIndex, T::Hash>;

	// Store money pot with his hash
	#[pallet::storage]
	#[pallet::getter(fn money_pots)]
	pub type MoneyPots<T: Config> = StorageMap<_, Twox64Concat, T::Hash, MoneyPot<T>>;

	// Store money pots owned by an account
	#[pallet::storage]
	#[pallet::getter(fn money_pot_owned)]
	pub type MoneyPotOwned<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::Hash, T::MaxMoneyPotCurrentlyOpen>,
		ValueQuery,
	>;

	// Store contributions to a given money pot
	#[pallet::storage]
	#[pallet::getter(fn money_pot_contribution)]
	pub(super) type MoneyPotContribution<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::Hash,
		BoundedVec<(T::AccountId, BalanceOf<T>), T::MaxMoneyPotContributors>,
		ValueQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub money_pot: Vec<(T::AccountId, T::AccountId, BalanceOf<T>)>, //Vec<MoneyPot<T>>, //Vec<(T::AccountId, [u8; 16], Gender)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig { money_pot: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (sender, receiver, amount) in &self.money_pot {
				MoneyPot::<T>::create(sender, receiver).with_native_currency_limit(amount.clone());
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100)]
		pub fn create_with_limit_amount(
			origin: OriginFor<T>,
			receiver: T::AccountId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			log::info!("💰 [Money pot] - create_with_limit_amount call");
			let sender = ensure_signed(origin)?;

			// Amount should be at least equal to min contribution and sup than 0
			ensure!(
				amount.saturated_into::<u32>() > 0u32 && 
				amount >= T::MinContribution::get(), <Error<T>>::AmountToLow);

			log::info!("💰 [Money pot] - create_with_limit_amount ok ensure_signed");

			let mut created_money_pot = MoneyPot::<T>::create(&sender, &receiver);

			created_money_pot.with_native_currency_limit(amount);
			let ref_hash = Self::control_creation(created_money_pot)?;
			log::info!("💰 [Money pot] - create_with_limit_amount ok control_creation");

			log::info!("💰 [Money pot] - Money pot created with_native_currency_limit");
			Self::deposit_event(Event::Created { ref_hash });

			Ok(())
		}

		#[pallet::weight(100)]
		pub fn create_with_limit_block(
			origin: OriginFor<T>,
			receiver: T::AccountId,
			end_block: T::BlockNumber,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Check max block number to avoid a schedule in 10 000 years...
			let current_block = <frame_system::Pallet<T>>::block_number();

			log::info!("💰 [Money pot] - create_with_limit_block - Current block = {:?} / End block = {:?} / Max block = {:?} / current_block + T::MaxBlockNumberEndTime::get().into() = {:?}", current_block, end_block, T::MaxBlockNumberEndTime::get(), current_block + T::MaxBlockNumberEndTime::get().into());
			ensure!(
				end_block <= current_block + T::MaxBlockNumberEndTime::get().into(),
				<Error<T>>::ScheduleError
			);

			let mut created_money_pot = MoneyPot::<T>::create(&sender, &receiver);
			created_money_pot.with_end_block(end_block);
			let ref_hash = Self::control_creation(created_money_pot)?;

			log::info!("💰 [Money pot] - create_with_limit_block ok control_creation");

			let schedule_result = T::Scheduler::schedule_named(
				(ID_LOCK_MONEY_POT, ref_hash).encode(),
				DispatchTime::At(end_block),
				None,
				63,
				frame_system::RawOrigin::Root.into(),
				Call::transfer_balance { ref_hash }.into(),
			)
			.map_err(|_| <Error<T>>::ScheduleError)?;

			log::info!("💰 [Money pot] - schedule dispatch ok ({:?})", schedule_result);

			Self::deposit_event(Event::Created { ref_hash });

			Ok(())
		}

		#[pallet::weight(100)]
		pub fn add_balance(
			origin: OriginFor<T>,
			ref_hash: T::Hash,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// TODO: do I need to get my account with Lookup struct ?
			// let who2 = T::Lookup::lookup(who)?;

			let contributor = ensure_signed(origin)?;

			/* 	Check if amount is compatible with contribution step
					amount % T::StepContribution::get() give a BalanceOf<T>
					We need to cast this to u32 to check if the modulo is correct
			*/
			ensure!(
				(amount % T::StepContribution::get()).saturated_into::<u32>() == 0u32,
				<Error<T>>::InvalidAmountStep
			);

			// Check if amount is sup than min contribution
			ensure!(amount >= T::MinContribution::get(), <Error<T>>::AmountToLow);

			let money_pot = Self::get_money_pot(&ref_hash)?;
			ensure!(money_pot.is_active, <Error<T>>::MoneyPotIsClose);

			// Check if the sender has enought fund
			ensure!(
				T::Currency::free_balance(&contributor) >= amount + T::ExistentialDeposit::get(),
				<Error<T>>::NotEnoughBalance
			);

			log::info!(
				"💰 [Money pot] - Free balance = {:?}",
				T::Currency::free_balance(&contributor)
			);
			Self::lock_balance(&contributor, amount);

			// Insert contribution
			// If there is a previous contribution with the same account, the amount is added to the previous one
			<MoneyPotContribution<T>>::try_mutate(&ref_hash, |p| {
				if let Some(previous) = p.iter_mut().find(|prev| prev.0 == contributor) {
					previous.1 += amount;
					Ok(())
				} else {
					p.try_push((contributor.clone(), amount))
				}
			})
			.map_err(|_| <Error<T>>::MaxMoneyPotContributors)?;

			if Self::is_need_transfer(&ref_hash, money_pot)? {
				Self::transfer_contributions(ref_hash)?;
			}

			Self::deposit_event(Event::MoneyAdded { ref_hash, who: contributor, amount });

			Ok(())
		}

		// Transfer the money pot balance and close it
		// Can only be called by Scheduler and Root user
		#[pallet::weight(100)]
		pub fn transfer_balance(origin: OriginFor<T>, ref_hash: T::Hash) -> DispatchResult {
			ensure_root(origin).unwrap();
			Self::transfer_contributions(ref_hash)?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_money_pot(id: &T::Hash) -> Result<MoneyPot<T>, Error<T>> {
			ensure!(Self::check_if_money_pot_exists(id), <Error<T>>::DoesNotExists);
			Self::get_money_pot_from_hash(id)
		}

		/// Check if the potential owner is the real owner of the pot
		/// Return true if `potential_owner` is the owner
		///
		/// Can return Error::DoesNotExists if the `id` is invalid
		pub fn check_owner(potential_owner: &T::AccountId, id: &T::Hash) -> Result<bool, Error<T>> {
			Ok(Self::get_money_pot_from_hash(id)?.owner == *potential_owner)
		}

		/// Return true if the money pot exists
		fn check_if_money_pot_exists(id: &T::Hash) -> bool {
			// TODO : check if money pot always open
			Self::money_pots(id) != None
		}

		/// Return the money pot associated to the hash if exists
		/// Return Error::DoesNotExists otherwise
		fn get_money_pot_from_hash(id: &T::Hash) -> Result<MoneyPot<T>, Error<T>> {
			Self::money_pots(id).ok_or(<Error<T>>::DoesNotExists)
		}

		/// Ensure the money pot creation is valid
		/// Check if we haven't the same `id` previously stored
		/// Push the money pot into the owner storage and check if it doesn't exceed max money pot open
		pub fn control_creation(money_pot: MoneyPot<T>) -> Result<T::Hash, Error<T>> {
			// Unique ID
			let money_pot_hash = T::Hashing::hash_of(&money_pot);

			// Check if the money pot already exists
			ensure!(Self::money_pots(&money_pot_hash) == None, <Error<T>>::AlreadyExists);

			// Check if the owner have less than `MaxMoneyPotCurrentlyOpen` money pot currently active
			<MoneyPotOwned<T>>::try_mutate(&money_pot.owner, |pots| pots.try_push(money_pot_hash))
				.map_err(|_| <Error<T>>::MaxOpenOverflow)?;

			// Save
			<MoneyPots<T>>::insert(money_pot_hash, money_pot);

			// Inc money pot counter
			let pot_id = Self::money_pot_count() + 1;
			MoneyPotsCount::<T>::put(pot_id);
			// Also do the mapping between Id and Hash
			MoneyPotsHash::<T>::insert(pot_id, money_pot_hash);

			Ok(money_pot_hash)
		}

		/// Check if contributor has enough balance and balance can be locked
		pub fn lock_balance(contributor: &T::AccountId, amount: BalanceOf<T>) -> () {
			log::info!("💰 [Money pot] - Lock {:?} of {:?}", amount, contributor);
			T::Currency::set_lock(
				ID_LOCK_MONEY_POT,
				&contributor,
				amount,
				frame_support::traits::WithdrawReasons::TRANSFER,
			);
			log::info!("💰 [Money pot] - Lock done");
		}

		/// Return true if the end time has been reached and the funds can be transferred
		pub fn is_need_transfer(id: &T::Hash, money_pot: MoneyPot<T>) -> Result<bool, Error<T>> {
			// Sum all contribution and check if amount exceed money pot limit
			let contributions = Self::money_pot_contribution(id);

			let initial_balance: u128 = 0;
			let mut total_amount: BalanceOf<T> =
				SaturatedConversion::saturated_into::<BalanceOf<T>>(initial_balance);
			contributions.iter().for_each(|m| total_amount += m.1);
			log::info!("💰 [Money pot] - Current total amount : {:?}", total_amount);

			match money_pot.end_time {
				None => Err(<Error<T>>::NoEndTimeSpecified),
				Some(end_type) => match end_type {
					EndType::Time(_) => Ok(false),
					EndType::AmountReached { amount_type, amount } => {
						log::info!("💰 [Money pot] - Target amount : {:?}", amount);
						Ok(total_amount >= amount)
					},
				},
			}
		}

		/// Transfer funds and close money pot
		pub fn transfer_contributions(ref_hash: T::Hash) -> DispatchResult {
			log::info!("💰 [Money pot] - transfer_contributions has been called");
			// ensure_signed(origin)?;
			ensure!(Self::check_if_money_pot_exists(&ref_hash), <Error<T>>::DoesNotExists);

			let mut money_pot = Self::get_money_pot_from_hash(&ref_hash)?;
			let contributions = Self::money_pot_contribution(ref_hash);
			// TODO :
			// I need to check if I can withdraw for each account before call transfer
			// I need to check if the sum of all amount invested by all withdrawable account + receiver amount >= ExistentialDeposit

			for (contributor, amount) in contributions.iter() {
				log::info!(
					"💰 [Money pot] - {:?} has lock {:?} for money pot {:?}",
					contributor,
					amount,
					ref_hash
				);

				// For each contributor, we unlock the contribution and transfer is to be receiver
				T::Currency::remove_lock(ID_LOCK_MONEY_POT, &contributor);
				log::info!("💰 [Money pot] - Lock remove for {:?}", contributor);

				// Check if we can safely transfer amount after unclock
				ensure!(
					T::Currency::free_balance(&contributor) >= *amount,
					<Error<T>>::NotEnoughBalance
				);
			}

			for (contributor, amount) in contributions.iter() {
				T::Currency::transfer(
					&contributor,
					&money_pot.receiver,
					*amount,
					frame_support::traits::ExistenceRequirement::KeepAlive,
				)?;
				log::info!(
					"💰 [Money pot] - Transfer of {:?} from {:?} to {:?}",
					amount,
					contributor,
					money_pot.receiver
				);
			}

			// Time to close it
			money_pot.close();
			log::info!("💰 [Money pot] - Is close = {}", !money_pot.is_active);

			<MoneyPots<T>>::remove(&ref_hash);
			<MoneyPots<T>>::insert(ref_hash, money_pot);

			Self::deposit_event(Event::Closed { ref_hash });

			Ok(())
		}
	}
}
