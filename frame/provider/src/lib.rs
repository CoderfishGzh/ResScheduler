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
	use sp_hamster::{
		p_dapp::DAppInfo, p_deployment::DeploymentInfo, p_provider::ComputingResource, Balance,
	};

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

	/// 资源 index
	#[pallet::storage]
	#[pallet::getter(fn resource_index)]
	pub(super) type ResourceIndex<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// 资源信息
	#[pallet::storage]
	#[pallet::getter(fn resource)]
	pub(super) type Resources<T: Config> =
		StorageMap<_, Twox64Concat, u64, ComputingResource<T::AccountId>, OptionQuery>;

	/// 用户拥有的资源
	#[pallet::storage]
	#[pallet::getter(fn user_resources)]
	pub(super) type UserResources<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<u64>, OptionQuery>;

	/// 资源节点剩下的资源规格，从小到大进行排序，简单模拟资源调度分配
	/// 首先分配剩下资源最小且符合需求的资源
	/// (资源index， 资源规格(cpu + mem))
	#[pallet::storage]
	#[pallet::getter(fn resource_rank)]
	pub(super) type ResourceRank<T: Config> = StorageValue<_, (u64, u64), ValueQuery>;

	/// 部署信息index
	#[pallet::storage]
	#[pallet::getter(fn deployment_index)]
	pub(super) type DeploymentIndex<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// 部署信息
	#[pallet::storage]
	#[pallet::getter(fn deployment)]
	pub(super) type Deployments<T: Config> =
		StorageMap<_, Twox64Concat, u64, DeploymentInfo, OptionQuery>;

	/// DApp index
	#[pallet::storage]
	#[pallet::getter(fn dapp_index)]
	pub(super) type DAppIndex<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// DApp info
	#[pallet::storage]
	#[pallet::getter(fn dapps)]
	pub(super) type DApps<T: Config> = StorageMap<_, Twox64Concat, u64, DAppInfo, OptionQuery>;

	/// 用户拥有的DApp
	#[pallet::storage]
	#[pallet::getter(fn user_dapps)]
	pub(super) type UserDApps<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<u64>, OptionQuery>;

	// The genesis config type.
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub resource: Vec<(u64, ComputingResource<T::AccountId>)>,
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
	pub enum Event<T: Config> {
		// 部署DApp
		// (peer_id, cpu, memory, 启动方式 1是image:port 2是cid, command)
		DeploymentDApp(Vec<u8>, u8, u8, u8, Vec<u8>),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			T::DbWeight::get().reads_writes(1, 1)
		}
	}

	#[pallet::error]
	pub enum Error<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn register_resource(account_id: OriginFor<T>, _peer_id: Vec<u8>) -> DispatchResult {
			let _who = ensure_signed(account_id)?;
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn offline_resource(account_id: OriginFor<T>, _peer_id: Vec<u8>) -> DispatchResult {
			let _who = ensure_signed(account_id)?;
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn resource_heartbeat(account_id: OriginFor<T>, _peer_id: Vec<u8>) -> DispatchResult {
			let _who = ensure_signed(account_id)?;
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn request_dapp_deployment(
			account_id: OriginFor<T>,
			_peer_id: Vec<u8>,
		) -> DispatchResult {
			let _who = ensure_signed(account_id)?;
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn end_dapp_deployment(account_id: OriginFor<T>, _peer_id: Vec<u8>) -> DispatchResult {
			let _who = ensure_signed(account_id)?;
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn change_dapp_specification(
			account_id: OriginFor<T>,
			_peer_id: Vec<u8>,
		) -> DispatchResult {
			let _who = ensure_signed(account_id)?;
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn dapp_heartbeat(account_id: OriginFor<T>, _peer_id: Vec<u8>) -> DispatchResult {
			let _who = ensure_signed(account_id)?;
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {}
