#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use frame_support::{
	dispatch::DispatchResult, pallet_prelude::*, sp_runtime::traits::Convert, traits::Currency,
};

use frame_system::pallet_prelude::*;
use sp_std::{convert::TryInto, vec::Vec};

pub use pallet::*;
use sp_hamster::{
	p_dapp::{DAppInfo, DappStatus},
	p_deployment::DeploymentMethod,
	p_provider::ComputingResource,
};

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// 心跳间隔300个区块，即30分钟
/// 超过30分钟，判定为超时
const HEARTBEAT_INTERVAL: u32 = 300u32;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use sp_hamster::{
		p_dapp::DAppInfo,
		p_deployment::{DeploymentInfo, DeploymentMethod},
		p_provider::{ComputingResource, ResourceConfig, ResourceStatus},
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
	pub(super) type Resources<T: Config> = StorageMap<
		_,
		Twox64Concat,
		u64,
		ComputingResource<T::AccountId, T::BlockNumber>,
		OptionQuery,
	>;

	/// 用户拥有的资源
	#[pallet::storage]
	#[pallet::getter(fn user_resources)]
	pub(super) type UserResources<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<u64>, OptionQuery>;

	/// 资源节点剩下的资源规格，从小到大进行排序，简单模拟资源调度分配
	/// 首先分配剩下资源最小且符合需求的资源
	/// (资源规格(cpu + mem), 资源index)
	#[pallet::storage]
	#[pallet::getter(fn resource_rank)]
	pub(super) type ResourceRank<T: Config> = StorageValue<_, Vec<(u64, u64)>, ValueQuery>;

	/// 部署信息index
	#[pallet::storage]
	#[pallet::getter(fn deployment_index)]
	pub(super) type DeploymentIndex<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// 部署信息
	#[pallet::storage]
	#[pallet::getter(fn deployment)]
	pub(super) type Deployments<T: Config> =
		StorageMap<_, Twox64Concat, u64, DeploymentInfo<T::AccountId>, OptionQuery>;

	/// DApp index
	#[pallet::storage]
	#[pallet::getter(fn dapp_index)]
	pub(super) type DAppIndex<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// DApp info
	/// [index, info]
	#[pallet::storage]
	#[pallet::getter(fn dapps)]
	pub(super) type DApps<T: Config> =
		StorageMap<_, Twox64Concat, u64, DAppInfo<T::BlockNumber>, OptionQuery>;

	/// 用户拥有的DApp
	/// [user, dapp_name]
	#[pallet::storage]
	#[pallet::getter(fn user_dapps)]
	pub(super) type UserDApps<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Vec<Vec<u8>>, OptionQuery>;

	/// Dapp名字 对应的 index
	#[pallet::storage]
	#[pallet::getter(fn dappname_to_index)]
	pub(super) type DAppnameToIndex<T: Config> =
		StorageMap<_, Twox64Concat, Vec<u8>, u64, OptionQuery>;

	// The genesis config type.
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub resource: Vec<(u64, ComputingResource<T::AccountId, T::BlockNumber>)>,
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
		/// register resource success
		/// [AccountId, resource_index, cpu, memory]
		RegisterResourceSuccess(T::AccountId, u64, u8, u8),

		// 部署DApp
		// (peer_id, cpu, memory, 启动方式 1是image:port 2是cid, command, dapp_index)
		DeploymentDApp(Vec<u8>, u8, u8, u8, Vec<u8>, u64),

		/// 资源心跳
		/// [peer_id, dapps]
		ResourceHeartbeat(Vec<u8>, Vec<u64>),

		/// DApp 心跳
		/// [account_id, dapp_name]
		DAppHeartbeat(T::AccountId, Vec<u8>),

		/// 部署失败
		/// [部署失败的DApp name]
		DAppRedistribution(Vec<Vec<u8>>),

		/// 结束dapp成功
		/// [accoutId, dapp_name, dapp_index]
		EndDAppSuccess(T::AccountId, Vec<u8>, u64),

		/// 结束dapp服务
		/// [资源peer_id, dapp_index]
		StopDApp(Vec<u8>, u64),
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			// 获取资源列表
			let resource: Vec<(u64, ComputingResource<T::AccountId, T::BlockNumber>)> =
				Resources::<T>::iter().collect();

			// 检查获取心跳超时的dapp
			// 目前的做法是，将心跳超时的 dapp 直接停掉
			let dapps: Vec<(u64, DAppInfo<T::BlockNumber>)> = DApps::<T>::iter().collect();
			let failed_dapp_indexs = match Self::check_and_get_heartbeat_timeout(now, dapps) {
				Some(i) => i,
				None => Vec::new(),
			};
			// 处理心跳超时的 dapp
			Self::deal_timeout_dapps(failed_dapp_indexs);

			let failed_resource_index = match Self::check_heartbeat_timeout(now, resource) {
				Some(i) => i,
				None => return T::DbWeight::get().reads_writes(1, 1),
			};

			// 将下线资源的包含的DApp进行资源节点转移(留给offchainworker)

			// 处理资源信息(留给offchainworker)

			T::DbWeight::get().reads_writes(1, 1)
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// 存在名字重复的dapp
		RepeatDAppName,
		/// 实例化DApp失败
		InstantiateError,
		/// 无效的资源下标
		InvaildResourceIndex,
		/// 资源不属于用户
		ResourceNotOwnedByAccount,
		/// 用户没有部署过的dapp
		NotHaveDApp,
		/// 无效的dapp名字
		InvaildDAppName,
		/// dapp 重新分配资源部署失败
		DAppRedistributionError,
		/// 无效的dapp index
		InvaildDAppIndex,
		/// 处理下线资源信息出错
		ClearDownlineResourceInformation,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Demo 版本，没有做资源重复的处理
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn register_resource(
			account_id: OriginFor<T>,
			peer_id: Vec<u8>,
			public_ip: Vec<u8>,
			cpu: u8,
			memory: u8,
		) -> DispatchResult {
			let who = ensure_signed(account_id)?;

			let resource_index = ResourceIndex::<T>::get();
			let resource_config = ResourceConfig::new(cpu, memory);
			// get the current block height
			let block_number = <frame_system::Pallet<T>>::block_number();
			let computing_resource = ComputingResource::new(
				resource_index,
				who.clone(),
				peer_id,
				public_ip,
				resource_config,
				Vec::new(),
				ResourceStatus::Online,
				block_number,
			);

			// 更新索引
			ResourceIndex::<T>::put(resource_index.saturating_add(1));

			// 记录资源信息
			Resources::<T>::insert(resource_index, computing_resource);

			// 更新用户拥有的信息
			let mut user_resources =
				UserResources::<T>::get(who.clone()).unwrap_or_else(|| Vec::new());
			// 二分法找到插入下标
			if let Err(size) = user_resources.binary_search(&resource_index) {
				user_resources.insert(size, resource_index);
				UserResources::<T>::insert(who.clone(), user_resources);
			}

			// 更新资源排序
			let mut resource_rank = ResourceRank::<T>::get();
			if let Err(size) = resource_rank.binary_search(&((cpu + memory) as u64, resource_index))
			{
				resource_rank.insert(size, ((cpu + memory) as u64, resource_index));
				ResourceRank::<T>::set(resource_rank);
			}

			Self::deposit_event(Event::<T>::RegisterResourceSuccess(
				who,
				resource_index,
				cpu,
				memory,
			));

			Ok(())
		}

		/// 资源下线
		/// 当资源没有被使用，直接下线
		/// 当资源被使用了，将所有的dapp服务转移
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn offline_resource(account_id: OriginFor<T>, resource_index: u64) -> DispatchResult {
			let who = ensure_signed(account_id)?;

			// 判断资源是否存在
			ensure!(Resources::<T>::contains_key(resource_index), Error::<T>::InvaildResourceIndex,);

			let resource = Resources::<T>::get(resource_index).unwrap();

			// 判断资源是否属于who
			ensure!(resource.account_id == who.clone(), Error::<T>::ResourceNotOwnedByAccount,);

			// 处理资源包含的服务, 将服务重载至其他资源节点
			match Self::re_deal_dapps(resource.dapps) {
				None => {},
				Some(failed) => Self::deposit_event(Event::<T>::DAppRedistribution(failed)),
			}

			if !Self::clear_downline_resource_information(resource_index, who.clone()) {
				return Err(Error::<T>::ClearDownlineResourceInformation.into())
			}

			Ok(())
		}

		/// 资源进行心跳
		/// 更新资源心跳时间 以及 dapp心跳时间
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn resource_heartbeat(
			account_id: OriginFor<T>,
			resource_index: u64,
			dapps: Vec<u64>,
		) -> DispatchResult {
			let who = ensure_signed(account_id)?;

			ensure!(Resources::<T>::contains_key(resource_index), Error::<T>::InvaildResourceIndex,);

			let mut resource = Resources::<T>::get(resource_index).unwrap();

			// 判断资源是否属于用户
			ensure!(resource.account_id == who, Error::<T>::ResourceNotOwnedByAccount,);

			// 获取系统时间
			let block_number = <frame_system::Pallet<T>>::block_number();

			// 更新资源心跳时间
			resource.last_heartbeat = block_number;

			// 更新dapp心跳时间
			Self::update_dapp_heartbeat_time(dapps.clone());

			Self::deposit_event(Event::<T>::ResourceHeartbeat(resource.peer_id, dapps));
			Ok(())
		}

		/// 申请部署dapp，发送部署信息
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn request_dapp_deployment(
			account_id: OriginFor<T>,
			method: DeploymentMethod,
			dapp_name: Vec<u8>,
			cpu: u8,
			memory: u8,
			replicas: u8,
			acliable: u8,
		) -> DispatchResult {
			let who = ensure_signed(account_id)?;

			// 判断该dapp是否已经被该用户部署过
			if UserDApps::<T>::contains_key(who.clone()) {
				let user_dapps = UserDApps::<T>::get(who.clone()).unwrap();
				ensure!(!user_dapps.contains(dapp_name.as_ref()), Error::<T>::RepeatDAppName,)
			}

			// 生成部署信息
			let deployment =
				DeploymentInfo::new(who.clone(), method.clone(), cpu, memory, replicas, acliable);

			// 获取部署信息 index
			let deployment_index = DeploymentIndex::<T>::get();

			// 记录部署信息
			Deployments::<T>::insert(deployment_index, deployment);

			// 更新部署信息 index
			DeploymentIndex::<T>::put(deployment_index.saturating_add(1));

			// 实例化 Dapp
			let (resource_index, dapp_index) =
				match Self::instantiate(dapp_name.clone(), deployment_index, cpu, memory) {
					Some(info) => info,
					None => return Err(Error::<T>::InstantiateError.into()),
				};
			// 拿到 resource 的 peer_id
			let resource_peer_id = Resources::<T>::get(resource_index).unwrap().peer_id;

			// 更新用户的 实例DApp 列表
			// 将新的dapp名字更新到 实例列表里面
			let mut user_dapps = UserDApps::<T>::get(who.clone()).unwrap_or_else(|| Vec::new());
			if let Err(size) = user_dapps.binary_search(&dapp_name) {
				user_dapps.insert(size, dapp_name);
			}
			UserDApps::<T>::insert(who.clone(), user_dapps);

			// 判断部署方法
			let (m, command) = match &method {
				DeploymentMethod::Cli(c) => (1, c.clone()),
				DeploymentMethod::Ipfs(i) => (2, i.clone()),
			};

			// 资源(peer_id, cpu, memory, type, command, dapp_index)
			Self::deposit_event(Event::<T>::DeploymentDApp(
				resource_peer_id,
				cpu,
				memory,
				m,
				command,
				dapp_index,
			));

			Ok(())
		}

		/// 结束/下线 dapp
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn end_dapp_deployment(account_id: OriginFor<T>, dapp_index: u64) -> DispatchResult {
			let who = ensure_signed(account_id)?;

			// dapp 信息是否存在
			let dapp = match DApps::<T>::get(dapp_index) {
				Some(app) => app,
				None => return Err(Error::<T>::InvaildDAppIndex.into()),
			};

			let dapp_name = dapp.dapp_name;

			// 判断用户是否拥有这个dapp
			ensure!(UserDApps::<T>::contains_key(who.clone()), Error::<T>::NotHaveDApp,);

			let mut user_dapps = UserDApps::<T>::get(who.clone()).unwrap();

			ensure!(user_dapps.contains(dapp_name.as_ref()), Error::<T>::NotHaveDApp,);

			// 删除dapp相关的信息
			Self::clear_downline_dapp_information(who.clone(), dapp_index);

			Self::deposit_event(Event::<T>::EndDAppSuccess(
				who.clone(),
				dapp_name.clone(),
				dapp_index,
			));

			Ok(())
		}

		/// 修改dapp的规格
		/// 停掉旧的dapp，使用新的部署信息，重新部署dapp
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn change_dapp_specification(
			account_id: OriginFor<T>,
			method: DeploymentMethod,
			dapp_name: Vec<u8>,
			cpu: u8,
			memory: u8,
			replicas: u8,
			acliable: u8,
		) -> DispatchResult {
			let who = ensure_signed(account_id)?;

			//  判断该dapp是否被用户部署过
			if !UserDApps::<T>::contains_key(who.clone()) {
				return Err(Error::<T>::NotHaveDApp.into())
			}

			let user_dapps = UserDApps::<T>::get(who.clone()).unwrap();
			ensure!(user_dapps.contains(dapp_name.as_ref()), Error::<T>::NotHaveDApp,);

			// 拿到 dapp 的部署信息
			let old_dapp_index = DAppnameToIndex::<T>::get(dapp_name.clone()).unwrap();
			let dapp = DApps::<T>::get(old_dapp_index).unwrap();
			let old_deployment_index = dapp.method_index;

			// 删除部署信息
			Deployments::<T>::remove(old_deployment_index);

			// 删除dapp信息
			DApps::<T>::remove(old_dapp_index);

			// 从资源里删除对应的dapp index
			let mut resource = Resources::<T>::get(dapp.resource_index).unwrap();
			let resource_dapps: Vec<u64> =
				resource.dapps.into_iter().filter(|&dapp| dapp != old_dapp_index).collect();
			resource.dapps = resource_dapps;
			Resources::<T>::insert(dapp.resource_index, resource.clone());

			// 发送停止旧dapp服务的event
			Self::deposit_event(Event::<T>::StopDApp(resource.peer_id, old_dapp_index));

			// 生成新的部署信息
			let new_deployment =
				DeploymentInfo::new(who.clone(), method.clone(), cpu, memory, replicas, acliable);

			// 记录新的部署信息
			let deployment_index = DeploymentIndex::<T>::get();
			Deployments::<T>::insert(deployment_index, new_deployment);
			DeploymentIndex::<T>::put(deployment_index.saturating_add(1));

			// 实例化dapp
			let (resource_index, dapp_index) =
				match Self::instantiate(dapp_name.clone(), deployment_index, cpu, memory) {
					Some(info) => info,
					None => return Err(Error::<T>::InstantiateError.into()),
				};
			// 拿到 resource 的 peer_id
			let resource_peer_id = Resources::<T>::get(resource_index).unwrap().peer_id;

			// 判断部署方法
			let (m, command) = match &method {
				DeploymentMethod::Cli(c) => (1, c.clone()),
				DeploymentMethod::Ipfs(i) => (2, i.clone()),
			};

			// 资源(peer_id, cpu, memory, type, command, dapp_index)
			Self::deposit_event(Event::<T>::DeploymentDApp(
				resource_peer_id,
				cpu,
				memory,
				m,
				command,
				dapp_index,
			));

			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn dapp_heartbeat(account_id: OriginFor<T>, dapp_name: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(account_id)?;

			// 判断是否拥有这个dapp
			ensure!(UserDApps::<T>::contains_key(who.clone()), Error::<T>::NotHaveDApp,);

			let user_dapps = UserDApps::<T>::get(who.clone()).unwrap();

			let (dapp_index, mut dapp) = match user_dapps.binary_search(dapp_name.as_ref()) {
				Ok(size) => {
					let dapp_index = DAppnameToIndex::<T>::get(&user_dapps[size]).unwrap();
					let dapp = DApps::<T>::get(&dapp_index).unwrap();
					(dapp_index, dapp)
				},
				Err(_) => return Err(Error::<T>::InvaildDAppName.into()),
			};

			// 更新心跳时间
			let block_number = <frame_system::Pallet<T>>::block_number();
			dapp.last_heartbeat = block_number;

			// 记录dapp信息
			DApps::<T>::insert(dapp_index, dapp);

			Self::deposit_event(Event::<T>::DAppHeartbeat(who, dapp_name));
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// 实例化 Dapp
	/// 返回 Some(resource_index, dapp_index)
	fn instantiate(
		dapp_name: Vec<u8>,
		deployment_index: u64,
		cpu: u8,
		memory: u8,
	) -> Option<(u64, u64)> {
		// 分配资源节点
		let resource_index = match Self::allocate_resource_node(cpu, memory) {
			Some(index) => index,
			None => return None,
		};

		// 分配 dapp index
		let dapp_index = DAppIndex::<T>::get();
		DAppIndex::<T>::put(dapp_index + 1);

		// 获取时间
		let block_number = <frame_system::Pallet<T>>::block_number();

		// 创建 dapp
		let dapp = DAppInfo::new(
			dapp_name.clone(),
			deployment_index,
			resource_index,
			DappStatus::Online,
			block_number,
		);

		// 记录 dapp信息
		DApps::<T>::insert(dapp_index, dapp);

		// 记录\更新 dapp 名字 对应的 index
		DAppnameToIndex::<T>::insert(dapp_name, dapp_index);

		Some((resource_index, dapp_index))
	}

	/// 将 DApp 重新处理，分发到其他资源节点运行
	/// 返回分发失败的 DApp名字
	fn re_deal_dapps(dapps: Vec<u64>) -> Option<Vec<Vec<u8>>> {
		// 调度失败的名单
		let mut failed_dapps = Vec::new();

		for dapp_index in dapps {
			// 拿到 dapp 信息
			let dapp = DApps::<T>::get(dapp_index).unwrap();

			let dapp_name = dapp.dapp_name;

			let method_index = dapp.method_index;

			let method = Deployments::<T>::get(method_index).unwrap();

			let (resource_index, dapp_index) =
				match Self::instantiate(dapp_name.clone(), method_index, method.cpu, method.memory)
				{
					Some(info) => info,
					None => {
						failed_dapps.push(dapp_name.clone());
						continue
					},
				};

			// 获取资源peer_id
			let resource_peer_id = Resources::<T>::get(resource_index).unwrap().peer_id;

			// 判断部署方法
			let (m, command) = match &method.method {
				DeploymentMethod::Cli(c) => (1, c.clone()),
				DeploymentMethod::Ipfs(i) => (2, i.clone()),
			};

			// 资源(peer_id, cpu, memory, type, command, dapp_index)
			Self::deposit_event(Event::<T>::DeploymentDApp(
				resource_peer_id,
				method.cpu,
				method.memory,
				m,
				command,
				dapp_index,
			));
		}

		if failed_dapps.is_empty() {
			return None
		}

		Some(failed_dapps)
	}

	/// 返回分配的资源节点
	/// 更新 Resource，ResourceRank
	fn allocate_resource_node(cpu: u8, memory: u8) -> Option<u64> {
		// 拿到资源排序表
		let mut resource_rank = ResourceRank::<T>::get();

		let mut location = 0;
		let mut resource_node_index = 0;
		let mut enable = false;

		let mut ret = Resources::<T>::get(resource_rank[0].1).unwrap();

		for (i, (_, node_index)) in resource_rank.iter().enumerate() {
			// 寻找满足要求的节点
			let mut resource = Resources::<T>::get(node_index).unwrap();
			if resource.config.use_resource(cpu, memory) {
				// 记录下标
				location = i;
				// 记录node index
				resource_node_index = resource.index;
				// 找到符合分配的节点
				enable = true;
				// 记录该节点
				ret = resource;
				break
			}
		}

		log::info!("enable {:?}", enable);

		// 没有找到符合的节点
		if !enable {
			return None
		}

		// 删除该节点，从新排序
		resource_rank.remove(location);
		if let Err(size) = resource_rank.binary_search(&(
			(ret.config.unused_cpu + ret.config.unused_memory) as u64,
			resource_node_index,
		)) {
			resource_rank.insert(
				size,
				((ret.config.unused_cpu + ret.config.unused_memory) as u64, resource_node_index),
			);
		}
		// 更新存储
		ResourceRank::<T>::put(resource_rank);
		Resources::<T>::insert(resource_node_index, ret);

		Some(resource_node_index)
	}

	// 检查超时心跳的资源
	// 返回超时的资源节点
	fn check_heartbeat_timeout(
		now: T::BlockNumber,
		resources: Vec<(u64, ComputingResource<T::AccountId, T::BlockNumber>)>,
	) -> Option<Vec<u64>> {
		let mut ret: Vec<u64> = Vec::new();
		for (resouece_idnex, resource) in resources {
			let last_heartbeat = resource.last_heartbeat;
			let interval = now - last_heartbeat;
			// 超时
			if interval > HEARTBEAT_INTERVAL.into() {
				// 统计资源index
				ret.push(resouece_idnex);
			}
		}

		if ret.is_empty() {
			return None
		}
		Some(ret)
	}

	/// 处理下线资源
	/// * 从resource rank调度队列删除资源
	/// * 删除用户拥有的资源
	/// * 删除资源信息
	fn clear_downline_resource_information(resource_index: u64, who: T::AccountId) -> bool {
		if !Resources::<T>::contains_key(resource_index) {
			return false
		}

		// 从 resource rank 调度队列删除该资源
		// maybe bug
		let resource_rank = ResourceRank::<T>::get()
			.into_iter()
			.filter(|r| r.1 != resource_index)
			.collect::<Vec<(u64, u64)>>();
		ResourceRank::<T>::put(resource_rank);

		// 删除用户拥有的资源
		let mut user_resources = UserResources::<T>::get(who.clone()).unwrap();
		if let Ok(size) = user_resources.binary_search(&resource_index) {
			user_resources.remove(size);
		}
		UserResources::<T>::insert(who.clone(), user_resources);

		// 删除资源信息
		Resources::<T>::remove(resource_index);

		true
	}

	/// 更新dapp的上一次心跳时间
	fn update_dapp_heartbeat_time(dapps: Vec<u64>) {
		// 获取时间
		let block_number = <frame_system::Pallet<T>>::block_number();
		for dapp_index in dapps {
			let mut dapp = match DApps::<T>::get(dapp_index) {
				None => continue,
				Some(app) => app,
			};

			dapp.last_heartbeat = block_number;
			DApps::<T>::insert(dapp_index, dapp);
		}
	}

	fn check_and_get_heartbeat_timeout(
		now: T::BlockNumber,
		dapps: Vec<(u64, DAppInfo<T::BlockNumber>)>,
	) -> Option<Vec<u64>> {
		let mut ret: Vec<u64> = Vec::new();

		for (dapp_index, dapp) in dapps {
			let last_heartbeat = dapp.last_heartbeat;
			let interval = now - last_heartbeat;
			// 超时
			if interval > HEARTBEAT_INTERVAL.into() {
				// 统计资源index
				ret.push(dapp_index);
			}
		}

		if ret.is_empty() {
			return None
		}

		Some(ret)
	}

	/// 删除dapp相关的信息
	fn clear_downline_dapp_information(who: T::AccountId, dapp_index: u64) {
		// 获取dapp
		let dapp = DApps::<T>::get(dapp_index).unwrap();

		// 获取用户拥有的DApps
		let mut user_dapps = UserDApps::<T>::get(who.clone()).unwrap();

		// 从用户拥有的DApps删除该dapp
		user_dapps = user_dapps
			.into_iter()
			.filter(|d| d != &dapp.dapp_name)
			.collect::<Vec<Vec<u8>>>();

		UserDApps::<T>::insert(who.clone(), user_dapps);

		// 将 DApp 信息删除
		DApps::<T>::remove(dapp_index);

		// 将 DApp name 和 index 映射 删除
		DAppnameToIndex::<T>::remove(dapp.dapp_name.clone());

		// 从资源的包含的DApp删除
		let resource_index = dapp.resource_index;
		let mut resource = Resources::<T>::get(resource_index).unwrap();
		let resource_dapps = resource
			.dapps
			.into_iter()
			.filter(|index| index != &resource_index)
			.collect::<Vec<u64>>();
		resource.dapps = resource_dapps;
		Resources::<T>::insert(resource_index, resource);
	}

	/// 处理心跳超时的dapp
	fn deal_timeout_dapps(dapps: Vec<u64>) {}
}
