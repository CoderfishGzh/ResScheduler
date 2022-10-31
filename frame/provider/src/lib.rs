#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use frame_support::{
	dispatch::DispatchResult, pallet_prelude::*, sp_runtime::traits::Convert, traits::Currency,
};
use frame_system::pallet_prelude::*;
use sp_std::{convert::TryInto, vec::Vec};

pub use pallet::*;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use sp_hamster::{p_provider, p_provider::ComputingResource, Balance};

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// currency to pay fees and hold balances
		type Currency: Currency<Self::AccountId>;

		/// amount converted to numbers
		type BalanceToNumber: Convert<BalanceOf<Self>, u128>;

		type NumberToBalance: Convert<u128, BalanceOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// resource information
	#[pallet::storage]
	#[pallet::getter(fn resource)]
	pub(super) type Resources<T: Config> = StorageMap<
		_,
		Twox64Concat,
		u64,
		ComputingResource<T::BlockNumber, T::AccountId>,
		OptionQuery,
	>;

	/// resource index
	#[pallet::storage]
	#[pallet::getter(fn resource_index)]
	pub(super) type ResourceIndex<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// resource provider and resource association
	#[pallet::storage]
	#[pallet::getter(fn provider)]
	pub(super) type Providers<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<u64>, OptionQuery>;

	// The genesis config type.
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub resource: Vec<(u64, ComputingResource<T::BlockNumber, T::AccountId>)>,
		pub resource_index: u64,
	}

	// The default value for the genesis config type.
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { resource: Default::default(), resource_index: Default::default() }
		}
	}

	// The build of genesis for the pallet.
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			<ResourceIndex<T>>::put(&self.resource_index);
			for (a, b) in &self.resource {
				<Resources<T>>::insert(a, b);
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			T::DbWeight::get().reads_writes(1, 1)
		}
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

impl<T: Config> Pallet<T> {}
