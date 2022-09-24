#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

// ./target/release/node-money-pot --dev
// ./target/release/node-money-pot purge-chain --dev

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode};
	use frame_support::{
		pallet_prelude::{DispatchResult, *},
		sp_runtime::traits::Hash,
		sp_runtime::SaturatedConversion,
		traits::{IsType, Currency, LockableCurrency, LockIdentifier},
		Blake2_128Concat, BoundedVec, Twox64Concat,
	};
	use frame_system::{ensure_signed, pallet_prelude::*, ensure_root};
	use scale_info::TypeInfo;

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
    type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    /*
    * Coming soon :
    *   Add a minimim price for each contribution
    *   Add a step (10 by 10 for exemple)
    *   Keep track of all the money pot created (like kitties)
	* 	Implement money pot to be closed with a BlockNumber specified
	* 	Add ManualClose functionnality to allow money pot creator to close the money pot manually
	*	Add a vault where every funds will be transfered to instead of lock amount in every account (which seems less secure)
    */

    const ID_LOCK_MONEY_POT: LockIdentifier = *b"moneypot";

	// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[derive(RuntimeDebug, Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq)]
	#[scale_info(skip_type_params(T))]
	pub struct MoneyPot<T: Config> {
		/// The money pot identifier
		//pub id: Hash,
		/// The creator
		pub owner: AccountOf<T>,
		/// Person who will receive fund
		pub receiver: AccountOf<T>,
		/// When the pot will start
		pub start_time: T::BlockNumber,
		/// When the pot will end and funds transfer to receiver
		pub end_time: Option<EndType<T>>,
	}

	impl<T: Config> MoneyPot<T> {
		fn create(owner: &T::AccountId, receiver: &T::AccountId) -> MoneyPot<T> {
			log::info!("Money pot create called");

			MoneyPot::<T> {
				owner: owner.clone(),
				receiver: receiver.clone(),
				start_time: <frame_system::Pallet<T>>::block_number(),
				end_time: None
			}
		}

		pub fn with_native_currency_limit(&mut self, amount: BalanceOf<T>) {
			self.end_time =
				Some(EndType::AmountReached { amount_type: AmountType::NATIVE, amount });
		}

		pub fn with_end_block(&mut self, end_time: T::BlockNumber) {
			self.end_time = Some(EndType::Time(end_time));
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

	#[derive(RuntimeDebug, Clone, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq)]
	pub enum AmountType {
		NATIVE,
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
        // type LockableCurrency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		/// The maximum money pot can be currently open by an account
		#[pallet::constant]
		type MaxMoneyPotCurrentlyOpen: Get<u32>;

		/// The maximum contributors for each money pot
		#[pallet::constant]
		type MaxMoneyPotContributors: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Money pot has been created
		Created,
		/// The money has been transfered to the receiver
		Transfered,
		/// Money has been added
		MoneyAdded,
		/// Money has been locked
		MoneyLocked,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The account has more than 'MaxMoneyPotCurrentlyOpen' money pot
		MaxOpenOverflow,
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
		TransferFailed,
	}

	// #[pallet::storage]
	// pub(super) type StorageMoneyPot<T: Config> =
	// 	StorageMap<_, Blake2_128Concat, T::Hash, (T::AccountId, T::BlockNumber)>;

    // Storage
	#[pallet::storage]
	#[pallet::getter(fn money_pots)]
	pub(super) type MoneyPots<T: Config> = StorageMap<_, Twox64Concat, T::Hash, MoneyPot<T>>;

	#[pallet::storage]
	#[pallet::getter(fn money_pot_owned)]
	pub(super) type MoneyPotOwned<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::Hash, T::MaxMoneyPotCurrentlyOpen>,
		ValueQuery,
	>;

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
            log::info!("create_with_limit_amount call");
			let sender = ensure_signed(origin)?;

            log::info!("create_with_limit_amount ok ensure_signed");

			let mut created_money_pot = MoneyPot::<T>::create(&sender, &receiver);
			created_money_pot.with_native_currency_limit(amount);
			let id = Self::control_creation(created_money_pot)?;
            log::info!("create_with_limit_amount ok control_creation");

			log::info!("Money pot created with_native_currency_limit");
			Self::deposit_event(Event::Created);

			Ok(())
		}

		#[pallet::weight(100)]
		pub fn create_with_limit_block(
			origin: OriginFor<T>,
			receiver: T::AccountId,
			end_block: T::BlockNumber,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let mut created_money_pot = MoneyPot::<T>::create(&sender, &receiver);
			created_money_pot.with_end_block(end_block);
			let id = Self::control_creation(created_money_pot)?;

			Self::deposit_event(Event::Created);

			Ok(())
		}

		#[pallet::weight(100)]
		pub fn add_funds_to_pot(origin: OriginFor<T>, money_pot_id: T::Hash, amount: BalanceOf<T>) -> DispatchResult {
            // TODO: do I need to get my account with Lookup struct and check ExistentialDeposit ?
			// let who = T::Lookup::lookup(who)?;
			// let existential_deposit = T::ExistentialDeposit::get();

			let contributor = ensure_signed(origin)?;
			let money_pot = Self::get_money_pot(&money_pot_id)?;

            // Check if the sender has enought fund
			ensure!(T::Currency::free_balance(&contributor) >= amount, <Error<T>>::NotEnoughBalance);

            log::info!("Free balance = {:?}", T::Currency::free_balance(&contributor));
			Self::lock_balance(&contributor, amount);

            // let contributions = <MoneyPotContribution<T>>::try_get(&money_pot_id).unwrap_or()
            <MoneyPotContribution<T>>::try_mutate(&money_pot_id, |p| {
                p.try_push((contributor.clone(), amount))
            }).map_err(|_| <Error<T>>::MaxMoneyPotContributors)?;

			if Self::is_need_transfer(&money_pot_id, money_pot)? {
				Self::transfer_contributions(&money_pot_id)?;
			}

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
            // TODO : check if money pot always open
            Self::money_pots(id).ok_or(<Error<T>>::DoesNotExists)
        }

		/// Ensure the money pot creation is valid
		/// Check if we haven't the same `id` previously stored
		/// Push the money pot into the owner storage and check if it doesn't exceed max money pot open
		pub fn control_creation(money_pot: MoneyPot<T>) -> Result<T::Hash, Error<T>> {
			// Unique ID
			let money_pot_id = T::Hashing::hash_of(&money_pot);

			// Check if the money pot already exists
			ensure!(Self::money_pots(&money_pot_id) == None, <Error<T>>::AlreadyExists);

			// Check if the owner have less than `MaxMoneyPotCurrentlyOpen` money pot currently active
			<MoneyPotOwned<T>>::try_mutate(&money_pot.owner, |pots| pots.try_push(money_pot_id))
				.map_err(|_| <Error<T>>::MaxOpenOverflow)?;

            // Save
            <MoneyPots<T>>::insert(money_pot_id, money_pot);

			Ok(money_pot_id)
		}

		/// Check if contributor has enough balance and balance can be locked
		pub fn lock_balance(contributor: &T::AccountId, amount: BalanceOf<T>) -> () {
			log::info!("Lock {:?} of {:?}", amount, contributor);
			T::Currency::set_lock(ID_LOCK_MONEY_POT, &contributor, amount, frame_support::traits::WithdrawReasons::TRANSFER);
			log::info!("Lock done");
			Self::deposit_event(Event::MoneyLocked);
		}

		/// Return true if the end time has been reached and the funds can be transferred
		pub fn is_need_transfer(id: &T::Hash, money_pot: MoneyPot<T>) -> Result<bool, Error<T>> {
			// Sum all contribution and check if amount exceed money pot limit
			let contributions = Self::money_pot_contribution(id);

			let initial_balance: u128 = 0;
			let mut total_amount: BalanceOf<T> = SaturatedConversion::saturated_into::<BalanceOf<T>>(initial_balance);
			contributions.iter().for_each(|m| total_amount += m.1);
			log::info!("Current total amount : {:?}", total_amount);

			match money_pot.end_time {
				None => Err(<Error<T>>::NoEndTimeSpecified),
				Some(end_type) => {
					match end_type {
						EndType::Time(block_number) => {
							// TODO
							Ok(false)
						},
						EndType::AmountReached { amount_type, amount} => {
							log::info!("Target amount : {:?}", amount);
							Ok(total_amount >= amount)
						},
					}
				},
			}
		}

		/// Transfer funds and close money pot
		pub fn transfer_contributions(id: &T::Hash) -> DispatchResult {
			log::info!("transfer_contributions has been called");
			// ensure_signed(origin)?;
			ensure!(Self::check_if_money_pot_exists(&id), <Error<T>>::DoesNotExists);

			let money_pot = Self::get_money_pot_from_hash(&id)?;

			// TODO :
			// I need to check if I can withdraw for each account before call transfer
			// I need to check if the sum of all amount invested by all withdrawable account + receiver amount >= ExistentialDeposit

			for (contributor, amount) in Self::money_pot_contribution(id).iter() {
				log::info!("{:?} has lock {:?} for money pot {:?}", contributor, amount, id);
				// For each contributor, we unlock the contribution and transfer is to be receiver
				T::Currency::remove_lock(ID_LOCK_MONEY_POT, &contributor);
				log::info!("Lock remove for {:?}", contributor);
				T::Currency::transfer(&contributor, &money_pot.receiver, *amount, frame_support::traits::ExistenceRequirement::KeepAlive)?;//.map_err(|_| <Error<T>>::TransferFailed)?;
				log::info!("Transfer of {:?} from {:?} to {:?}", amount, contributor, money_pot.receiver);
			}

			Ok(())
		}


	}
}
