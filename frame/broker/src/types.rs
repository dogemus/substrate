use crate::{Config, CoretimeInterface, CoreIndex, CorePart, CoreAssignment, TaskId};
use codec::{Encode, Decode, MaxEncodedLen};
use scale_info::TypeInfo;
use frame_support::traits::fungible::Inspect;
use frame_system::Config as SConfig;
use sp_arithmetic::Perbill;
use sp_core::{ConstU32, RuntimeDebug};
use sp_runtime::BoundedVec;

pub type BalanceOf<T> = <<T as Config>::Currency as Inspect<<T as SConfig>::AccountId>>::Balance;
pub type RelayBalanceOf<T> = <<T as Config>::Coretime as CoretimeInterface>::Balance;
pub type RelayBlockNumberOf<T> = <<T as Config>::Coretime as CoretimeInterface>::BlockNumber;
pub type RelayAccountIdOf<T> = <<T as Config>::Coretime as CoretimeInterface>::AccountId;

/// Relay-chain block number with a fixed divisor of Config::TimeslicePeriod.
pub type Timeslice = u32;
/// Counter for the total number of set bits over every core's `CorePart`. `u32` so we don't
/// ever get an overflow.
pub type PartCount = u32;
/// The same as `PartCount` but signed.
pub type SignedPartCount = i32;

/// Self-describing identity for a Region of Bulk Coretime.
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct RegionId {
	/// The timeslice at which this Region begins.
	pub begin: Timeslice,
	/// The index of the Polakdot Core on which this Region will be scheduled.
	pub core: CoreIndex,
	/// The regularity parts in which this Region will be scheduled.
	pub part: CorePart,
}
impl From<u128> for RegionId {
	fn from(x: u128) -> Self {
		Self {
			begin: (x >> 96) as u32,
			core: (x >> 80) as u16,
			part: x.into(),
		}
	}
}
impl From<RegionId> for u128 {
	fn from(x: RegionId) -> Self {
		(x.begin as u128) << 96 | (x.core as u128) << 80 | u128::from(x.part)
	}
}
#[test]
fn region_id_converts_u128() {
	let r = RegionId { begin: 0x12345678u32, core: 0xabcdu16, part: 0xdeadbeefcafef00d0123.into() };
	let u = 0x12345678_abcd_deadbeefcafef00d0123u128;
	assert_eq!(RegionId::from(u), r);
	assert_eq!(u128::from(r), u);
}

/// The rest of the information describing a Region.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct RegionRecord<AccountId, Balance> {
	/// The end of the Region.
	pub end: Timeslice,
	/// The owner of the Region.
	pub owner: AccountId,
	/// The amount paid to Polkadot for this Region.
	pub paid: Option<Balance>,
}
pub type RegionRecordOf<T> = RegionRecord<<T as SConfig>::AccountId, BalanceOf<T>>;

/// An distinct item which can be scheduled on a Polkadot Core.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ScheduleItem {
	/// The regularity parts in which this Item will be scheduled on the Core.
	pub part: CorePart,
	/// The job that the Core should be doing.
	pub assignment: CoreAssignment,
}
pub type Schedule = BoundedVec<ScheduleItem, ConstU32<80>>;

/// The record body of a Region which was contributed to the Instantaneous Coretime Pool. This helps
/// with making pro rata payments to contributors.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ContributionRecord<AccountId> {
	/// The end of the Region contributed.
	pub length: Timeslice,
	/// The identity of the contributor.
	pub payee: AccountId,
}
pub type ContributionRecordOf<T> = ContributionRecord<<T as SConfig>::AccountId>;

/// A per-timeslice bookkeeping record for tracking Instantaneous Coretime Pool activity and
/// making proper payments to contributors.
#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct InstaPoolHistoryRecord<Balance> {
	/// The total amount of Coretime (measured in Regularity Parts or 1/80th of a single block
	/// of a Polkadot Core) contributed over a timeslice minus any contributions which have
	/// already been paid out.
	pub total_contributions: PartCount,
	/// The total amount of Coretime (measured in Regularity Parts or 1/80th of a single block
	/// of a Polkadot Core) contributed by the Polkadot System in this timeslice.
	pub system_contributions: PartCount,
	/// The payout remaining for the `total_contributions`, or `None` if the revenue is not yet
	/// known.
	pub maybe_payout: Option<Balance>,
}
pub type InstaPoolHistoryRecordOf<T> = InstaPoolHistoryRecord<BalanceOf<T>>;

/// How much of a core has been assigned or, if completely assigned, the workload itself.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum CompletionStatus {
	/// The core is not fully assigned; the inner is the parts which have.
	Partial(CorePart),
	/// The core is fully assigned; the inner is the workload which has been assigned.
	Complete(Schedule),
}
impl CompletionStatus {
	/// Return reference to the complete workload, or `None` if incomplete.
	pub fn complete(&self) -> Option<&Schedule> {
		match self {
			Self::Complete(s) => Some(s),
			Self::Partial(_) => None,
		}
	}
	/// Return the complete workload, or `None` if incomplete.
	pub fn drain_complete(self) -> Option<Schedule> {
		match self {
			Self::Complete(s) => Some(s),
			Self::Partial(_) => None,
		}
	}
}

/// A record of an allowed renewal.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct AllowedRenewalRecord<Balance> {
	/// The timeslice denoting the beginning of the Region for which a renewal can secure.
	pub begin: Timeslice,
	/// The price for which the next renewal can be made.
	pub price: Balance,
	/// The workload which will be scheduled on the Core in the case a renewal is made, or if
	/// incomplete, then the parts of the core which have been scheduled.
	pub completion: CompletionStatus,
}
pub type AllowedRenewalRecordOf<T> = AllowedRenewalRecord<BalanceOf<T>>;

/// General status of the system.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct StatusRecord {
	/// The total number of cores which can be assigned (one plus the maximum index which can
	/// be used in `Coretime::assign`).
	pub core_count: CoreIndex,
	/// The current size of the Instantaneous Coretime Pool, measured in
	/// Regularity Parts or 1/80th of a single block of a Polkadot Core.
	pub pool_size: PartCount,
	/// The current amount of the Instantaneous Coretime Pool which is provided by the Polkadot
	/// System, rather than provided as a result of privately operated Coretime.
	pub system_pool_size: PartCount,
	/// The last (Relay-chain) timeslice which we processed for (this processing is generally
	/// done some number of timeslices in advance of actual Relay-chain execution to make up
	/// for latencies and any needed Relay-side preparations).
	pub last_timeslice: Timeslice,
}

/// A record of flux in the InstaPool.
#[derive(Encode, Decode, Clone, Copy, Default, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct PoolIoRecord {
	/// The total change of the pool, measured in Regularity Parts.
	pub total: SignedPartCount,
	/// The total change of the portion of the pool supplied by the Polkaot System,
	/// measured in Regularity Parts.
	pub system: SignedPartCount,
}

/// The status of a Bulk Coretime Sale.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct SaleInfoRecord<Balance, BlockNumber> {
	/// The local block number at which the sale will/did start.
	pub sale_start: BlockNumber,
	/// The length in blocks of the Leadin Period (where the price is decreasing).
	pub leadin_length: BlockNumber,
	/// The price of Bulk Coretime at the beginning of the Leadin Period.
	pub start_price: Balance,
	/// The price of Bulk Coretime by the end of the Leadin Period.
	pub reserve_price: Balance,
	/// The price at which the Bulk Coretime offered sold out, if it did sell out.
	pub sellout_price: Option<Balance>,
	/// The first timeslice of the Regions which are being sold in this sale.
	pub region_begin: Timeslice,
	/// The timeslice on which the Regions which are being sold in the sale terminate. (i.e. One
	/// after the last timeslice which the Regions control.)
	pub region_end: Timeslice,
	/// The index of the first core which is for sale. Core of Regions which are sold have
	/// incrementing indices from this.
	pub first_core: CoreIndex,
	/// The number of cores we want to sell, ideally. Selling this amount would result in no
	/// change to the reserve_price for the next sale.
	pub ideal_cores_sold: CoreIndex,
	/// Number of cores which are/have been offered for sale.
	pub cores_offered: CoreIndex,
	/// Number of cores which have been sold; never more than cores_offered.
	pub cores_sold: CoreIndex,
}
pub type SaleInfoRecordOf<T> = SaleInfoRecord<
	BalanceOf<T>,
	<T as SConfig>::BlockNumber,
>;

/// Record for Polkadot Core reservations (generally tasked with the maintenance of System
/// Chains).
pub type ReservationsRecord<Max> = BoundedVec<Schedule, Max>;
pub type ReservationsRecordOf<T> = ReservationsRecord<<T as Config>::MaxReservedCores>;

/// Information on a single legacy lease.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct LeaseRecordItem {
	pub until: Timeslice,
	pub task: TaskId,
}

/// Record for Polkadot Core legacy leases.
pub type LeasesRecord<Max> = BoundedVec<LeaseRecordItem, Max>;
pub type LeasesRecordOf<T> = LeasesRecord<<T as Config>::MaxLeasedCores>;

/// Configuration of this pallet.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ConfigRecord<BlockNumber> {
	/// The number of timeslices in advance which scheduling should be fixed and the
	/// `Coretime::assign` API used to inform the Relay-chain.
	pub advance_notice: Timeslice,
	/// The length in blocks of the Interlude Period for forthcoming sales.
	pub interlude_length: BlockNumber,
	/// The length in blocks of the Leadin Period for forthcoming sales.
	pub leadin_length: BlockNumber,
	/// The length in timeslices of Regions which are up for sale in forthcoming sales.
	pub region_length: Timeslice,
	/// The proportion of cores available for sale which should be sold in order for the price
	/// to remain the same in the next sale.
	pub ideal_bulk_proportion: Perbill,
	/// An artificial limit to the number of cores which are allowed to be sold. If `Some` then
	/// no more cores will be sold than this.
	pub limit_cores_offered: Option<CoreIndex>,
	/// The amount by which the renewal price increases each sale period.
	pub renewal_bump: Perbill,
	/// The duration by which rewards for contributions to the InstaPool must be collected.
	pub contribution_timeout: Timeslice,
}
pub type ConfigRecordOf<T> = ConfigRecord<
	<T as SConfig>::BlockNumber,
>;
