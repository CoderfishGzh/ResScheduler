use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;
use sp_std::vec::Vec;

/// ComputingResources
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ComputingResource<AccountId, BlockNumber> {
	// 资源索引
	pub index: u64,
	// 资源拥有者
	pub account_id: AccountId,
	// 指定资源p2p身份
	pub peer_id: Vec<u8>,
	// 资源公网ip
	pub public_ip: Vec<u8>,
	// 资源状态
	pub config: ResourceConfig,
	// 构建的服务组,记录 dapp index 组
	// 如果该资源挂掉，需要将所有服务转移到另外一个资源节点
	pub dapps: Vec<u64>,
	// 资源在线状态
	pub status: ResourceStatus,
	// 上一次心跳时间
	pub last_heartbeat: BlockNumber,
}

impl<AccountId, BlockNumber> ComputingResource<AccountId, BlockNumber> {
	pub fn new(
		index: u64,
		account_id: AccountId,
		peer_id: Vec<u8>,
		public_ip: Vec<u8>,
		config: ResourceConfig,
		dapps: Vec<u64>,
		status: ResourceStatus,
		last_heartbeat: BlockNumber,
	) -> Self {
		ComputingResource {
			index,
			account_id,
			peer_id,
			public_ip,
			config,
			dapps,
			status,
			last_heartbeat,
		}
	}

	// 增加DApp
	fn add_dapp(&mut self, dapp_index: u64) -> bool {
		match self.dapps.binary_search(&dapp_index) {
			Ok(_) => false,
			Err(size) => {
				self.dapps.insert(size, dapp_index);
				true
			},
		}
	}

	// 删除DApp
	fn remove_dapp(&mut self, dapp_index: u64) -> bool {
		match self.dapps.binary_search(&dapp_index) {
			Ok(size) => {
				self.dapps.remove(size);
				true
			},
			Err(_) => false,
		}
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ResourceStatus {
	// 在线
	Online,
	// 宕机
	Offline,
}

impl Default for ResourceStatus {
	fn default() -> Self {
		ResourceStatus::Online
	}
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ResourceConfig {
	// 总资源
	pub total_cpu: u8,
	pub total_memory: u8,
	// 剩下未使用的资源
	pub unused_cpu: u8,
	pub unused_memory: u8,
}

impl ResourceConfig {
	pub fn new(total_cpu: u8, total_memory: u8) -> Self {
		ResourceConfig {
			total_cpu,
			total_memory,
			unused_cpu: total_cpu,
			unused_memory: total_memory,
		}
	}

	// 使用资源
	pub fn use_resource(&mut self, cpu: u8, memory: u8) -> bool {
		if cpu > self.unused_cpu || memory > self.unused_memory {
			return false
		}
		self.unused_cpu = self.unused_cpu.saturating_sub(cpu);
		self.unused_memory = self.unused_memory.saturating_sub(memory);

		true
	}

	// 释放资源
	pub fn release_resource(&mut self, cpu: u8, memory: u8) -> bool {
		let used_cpu = self.total_cpu.saturating_sub(self.unused_cpu);
		let used_memory = self.total_memory.saturating_sub(self.unused_memory);

		if cpu > used_cpu || memory > used_memory {
			return false
		}

		self.unused_cpu = self.unused_cpu.saturating_add(cpu);
		self.unused_memory = self.unused_memory.saturating_add(memory);

		true
	}
}
