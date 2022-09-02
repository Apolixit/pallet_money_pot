#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {

	use frame_support::{
		pallet_prelude::{DispatchResult, *},
		storage::types::StorageMap,
		traits::IsType,
		Blake2_128Concat,
	};
	use frame_system::{ensure_signed, pallet_prelude::*};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		TodoEvent
	}

	#[pallet::error]
	pub enum Error<T> {
		TodoError,
	}

	#[pallet::storage]
	pub(super) type StorageTodo<T: Config> = StorageMap<_, Blake2_128Concat, T:: Hash, (T::AccountId, T::BlockNumber)>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn add_something(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::deposit_event(Event::TodoEvent);

			Ok(())
		}
	}
}
