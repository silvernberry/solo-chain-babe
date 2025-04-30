// This is free and unencumbered software released into the public domain.
//
// Anyone is free to copy, modify, publish, use, compile, sell, or
// distribute this software, either in source code form or as a compiled
// binary, for any purpose, commercial or non-commercial, and by any
// means.
//
// In jurisdictions that recognize copyright laws, the author or authors
// of this software dedicate any and all copyright interest in the
// software to the public domain. We make this dedication for the benefit
// of the public at large and to the detriment of our heirs and
// successors. We intend this dedication to be an overt act of
// relinquishment in perpetuity of all present and future rights to this
// software under copyright law.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
// OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
// ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
// OTHER DEALINGS IN THE SOFTWARE.
//
// For more information, please refer to <http://unlicense.org>

// Substrate and Polkadot dependencies
// Std / Core
use alloc::vec;

// External Crates
use parity_scale_codec::Decode;

// Frame Support
use frame_support::{
    derive_impl,
    pallet_prelude::{DispatchClass, Get},
    parameter_types,
    traits::{
        ConstBool, ConstU128, ConstU32, ConstU64, ConstU8,
        Nothing, VariantCountOf, EqualPrivilegeOnly
    },
    weights::{
        constants::{BlockExecutionWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
        IdentityFee, Weight,
    },
    PalletId,
};

// Frame System
use frame_system::{
    EnsureRoot,
    limits::{BlockLength, BlockWeights},
    offchain::{CreateInherent, CreateTransactionBase},
};

// Pallets
use pallet_contracts::config_preludes::{DefaultDepositLimit, DepositPerByte, DepositPerItem};
use pallet_election_provider_multi_phase::{
    self as election_provider_multi_phase, SolutionAccuracyOf,
};
use pallet_nomination_pools::adapter::TransferStake;
use pallet_session::PeriodicSessions;
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};

// Sp Runtime
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::H256;
use sp_runtime::{
    curve::PiecewiseLinear,
    traits::{Convert, One, OpaqueKeys},
    transaction_validity::TransactionPriority,
    FixedU128, Perbill,
};
use sp_staking::{EraIndex, SessionIndex};
use sp_version::RuntimeVersion;

// Election Provider Support
use frame_election_provider_support::{
    bounds::{CountBound, DataProviderBounds, ElectionBounds, SizeBound},
    onchain, BalancingConfig, ElectionDataProvider, SequentialPhragmen,
};

// Local Crate
use crate::{
    constants::{currency::*, time::*},
    SessionKeys, UncheckedExtrinsic,
};

// Local module imports
use super::{
	AccountId, Aura, Balance, NominationPools, Timestamp, Session, Balances, Staking, TransactionPayment, ElectionProviderMultiPhase, Block, BlockNumber, Hash, Nonce, PalletInfo, Runtime,
	RuntimeCall, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin, OriginCaller, RuntimeTask,
	System, EXISTENTIAL_DEPOSIT, SLOT_DURATION, VERSION,
};

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;

	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::with_sensible_defaults(
		Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
		NORMAL_DISPATCH_RATIO,
	);
	pub RuntimeBlockLength: BlockLength = BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub const SS58Prefix: u8 = 42;
}

/// The default types are being injected by [`derive_impl`](`frame_support::derive_impl`) from
/// [`SoloChainDefaultConfig`](`struct@frame_system::config_preludes::SolochainDefaultConfig`),
/// but overridden as needed.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
impl frame_system::Config for Runtime {
	/// The block type for the runtime.
	type Block = Block;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Version of the runtime.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub struct SignedDepositBase;
impl sp_runtime::traits::Convert<usize, Balance> for SignedDepositBase {
	fn convert(v: usize) -> Balance {
		// Assume 1 unit deposit base per voter
		(v as u128) * 1_000_000_000
	}
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<32>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
	type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Runtime>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
	type MaxAuthorities = ConstU32<32>;
	type MaxNominators = ConstU32<0>;
	type MaxSetIdSessionEntries = ConstU64<0>;

	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = DAYS;
	pub const VotingPeriod: BlockNumber = DAYS;
	pub const FastTrackVotingPeriod: BlockNumber = HOURS;
	pub const MinimumDeposit: Balance = 10 * DOLLARS;
	pub const EnactmentPeriod: BlockNumber = DAYS;
	pub const CooloffPeriod: BlockNumber = DAYS;
	pub const PreimageByteDeposit: Balance = 1 * MILLICENTS;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
	pub const MaxDeposits: u32 = 20;
	pub const MaxBlacklisted: u32 = 100;
	pub const InstantAllowed: bool = true;
}


impl pallet_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	
	type Scheduler = pallet_scheduler::Pallet<Runtime>;
	type Preimages = pallet_preimage::Pallet<Runtime>;
	type Currency = Balances;

	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = EnactmentPeriod; // Usually same as enactment.
	type MinimumDeposit = MinimumDeposit;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	type CooloffPeriod = CooloffPeriod;
	type MaxVotes = MaxVotes;
	type MaxProposals = MaxProposals;
	type MaxDeposits = MaxDeposits;
	type MaxBlacklisted = MaxBlacklisted;

	type ExternalOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type ExternalMajorityOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type ExternalDefaultOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type SubmitOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type FastTrackOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type InstantOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type CancellationOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type BlacklistOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type CancelProposalOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type VetoOrigin = frame_system::EnsureSigned<Self::AccountId>;

	type PalletsOrigin = OriginCaller;
	type Slash = ();
}


parameter_types! {
	pub const SchedulerMaxWeight: Weight = Weight::from_parts(80_000_000, 0);
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;

	type MaximumWeight = SchedulerMaxWeight;
	type ScheduleOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type MaxScheduledPerBlock = ConstU32<50>;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type Preimages = pallet_preimage::Pallet<Runtime>;
	type BlockNumberProvider = frame_system::Pallet<Runtime>;
}

pub struct BalanceToU256;
impl Convert<Balance, sp_core::U256> for BalanceToU256 {
	fn convert(balance: Balance) -> sp_core::U256 {
		sp_core::U256::from(balance)
	}
}
pub struct U256ToBalance;
impl Convert<sp_core::U256, Balance> for U256ToBalance {
	fn convert(n: sp_core::U256) -> Balance {
		n.try_into().unwrap_or(Balance::max_value())
	}
}

parameter_types! {
	pub const PostUnbondPoolsWindow: u32 = 4;
	pub const NominationPoolsPalletId: PalletId = PalletId(*b"py/nopls");
	pub const MaxPointsToBalance: u8 = 10;
}


impl pallet_nomination_pools::Config for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RewardCounter = FixedU128;
	type BalanceToU256 = BalanceToU256;
	type U256ToBalance = U256ToBalance;
	type PostUnbondingPoolsWindow = PostUnbondPoolsWindow;
	type MaxMetadataLen = ConstU32<256>;
	type MaxUnbonding = ConstU32<8>;
	type PalletId = NominationPoolsPalletId;
	type MaxPointsToBalance = MaxPointsToBalance;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type StakeAdapter =  TransferStake<Runtime, pallet_staking::Pallet<Runtime>>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type BlockNumberProvider =frame_system::Pallet<Runtime>;
	type Filter = frame_support::traits::Everything;
}

impl pallet_preimage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type ManagerOrigin = frame_system::EnsureRoot<AccountId>;
	type Consideration = ();
}

parameter_types! {
    pub const Period: u32 = 6 * DAYS; 
    pub const Offset: u32 = 0;        
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
    type ShouldEndSession = PeriodicSessions<Period, Offset>;
    type NextSessionRotation = PeriodicSessions<Period, Offset>;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
	type DisablingStrategy = (); 
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
	type MaxNominators = ConstU32<1000>;
	type MaxValidators = ConstU32<1000>;
}

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	pub const SessionsPerEra: SessionIndex = 6;
	pub const BondingDuration: EraIndex = 28;
	pub const SlashDeferDuration: EraIndex = 7;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxUnlockingChunks: u32 = 32;
	pub const MaxControllersInDeprecationBatch: u32 = 32;
	pub const MaxExposurePageSize: u32 = 4096;
}


impl pallet_staking::Config for Runtime {
	type OldCurrency = Balances;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CurrencyBalance = Balance;
	type UnixTime = Timestamp;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
	type ElectionProvider = ElectionProviderMultiPhase;
	type GenesisElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;	
	type NominationsQuota = pallet_staking::FixedNominationsQuota<16>;
	type HistoryDepth = ConstU32<84>;
	type RewardRemainder = ();
	type RuntimeEvent = RuntimeEvent;
	type Slash = ();
	type Reward = ();
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	type AdminOrigin = EnsureRoot<AccountId>;
	type SessionInterface = Self;
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = Session ;
	type MaxExposurePageSize = ConstU32<4096>;
	type VoterList = pallet_staking::UseNominatorsAndValidatorsMap<Self>;
	type TargetList = pallet_staking::UseValidatorsMap<Self>;
	type MaxUnlockingChunks = ConstU32<32>;
	type MaxControllersInDeprecationBatch = ConstU32<32>;
	type EventListeners = NominationPools ;
	type Filter = frame_support::traits::Everything;
	type BenchmarkingConfig = StakingBenchmarkingConfig;
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
}

parameter_types!{
		// phase durations. 1/4 of the last session for each.
		pub const SignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;
		pub const UnsignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;
		pub const BetterSignedThreshold: Perbill = Perbill::from_percent(10);
		pub OffchainRepeat: BlockNumber = 5;
		pub const MinerTxPriority: TransactionPriority = 100;
		pub const SignedRewardBase: Balance = 1 * DOLLARS;
		pub const SignedDepositByte: Balance = 1 * CENTS;

		pub MaxActiveValidators: u32 = 1000;
		pub const MaxBackersPerWinner: u32 = 50;
		pub const SignedMaxWeight: Weight = Weight::from_parts(500_000_000_000, 0);
		pub MaxElectingVoters: u32 = 40_000;


		pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
		*RuntimeBlockLength::get()
		.max
		.get(DispatchClass::Normal);
		// Solution can occupy 90% of normal block size
		pub MinerMaxWeight: Weight = RuntimeBlockWeights::get()
		.get(DispatchClass::Normal)
		.max_extrinsic.expect("Normal extrinsics have a weight limit configured; qed")
		.saturating_sub(BlockExecutionWeight::get());

}



impl pallet_election_provider_multi_phase::MinerConfig for Runtime {
	type AccountId = AccountId;
	type MaxLength = MinerMaxLength;
	type MaxWeight = MinerMaxWeight;
	type Solution = NposSolution16;
	type MaxVotesPerVoter =
	<<Self as pallet_election_provider_multi_phase::Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter;
	type MaxWinners = MaxActiveValidators;

	// The unsigned submissions have to respect the weight of the submit_unsigned call, thus their
	// weight estimate function is wired to this call's weight.
	fn solution_weight(v: u32, t: u32, a: u32, d: u32) -> Weight {
		<
			<Self as pallet_election_provider_multi_phase::Config>::WeightInfo
			as
			pallet_election_provider_multi_phase::WeightInfo
		>::submit_unsigned(v, t, a, d)
	}
}

/// Maximum number of iterations for balancing that will be executed in the embedded OCW
/// miner of election provider multi phase.
pub const MINER_MAX_ITERATIONS: u32 = 10;

pub struct OffchainRandomBalancing;
impl Get<Option<BalancingConfig>> for OffchainRandomBalancing {
	fn get() -> Option<BalancingConfig> {
		use sp_runtime::traits::TrailingZeroInput;
		let iterations = match MINER_MAX_ITERATIONS {
			0 => 0,
			max => {
				let seed = sp_io::offchain::random_seed();
				let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
					.expect("input is padded with zeroes; qed") %
					max.saturating_add(1);
				random as usize
			},
		};

		let config = BalancingConfig { iterations, tolerance: 0 };
		Some(config)
	}
}

frame_election_provider_support::generate_solution_type!(
	#[compact]
	pub struct NposSolution16::<
		VoterIndex = u32,
		TargetIndex = u16,
		Accuracy = sp_runtime::PerU16,
		MaxVoters = MaxElectingVoters,
	>(16)
);


pub struct OnChainSeqPhragmen;
impl onchain::Config for OnChainSeqPhragmen {
	type System = Runtime;
	type Solver = SequentialPhragmen<
		AccountId,
		pallet_election_provider_multi_phase::SolutionAccuracyOf<Runtime>,
	>;
	type DataProvider = <Runtime as pallet_election_provider_multi_phase::Config>::DataProvider;
	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
	type MaxWinners = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
	type Bounds =  ElectionBoundsWrapper ;
}

pub struct ElectionBoundsWrapper;

impl Get<ElectionBounds> for ElectionBoundsWrapper {
    fn get() -> ElectionBounds {
        ElectionBounds {
            voters: DataProviderBounds {
                count: Some(CountBound(5000)), 
                size: Some(SizeBound(1_000_000)), 
            },
            targets: DataProviderBounds {
                count: Some(CountBound(1250)), 
                size: Some(SizeBound(40_000)),
            },
        }
    }
}

pub struct ElectionProviderBenchmarkConfig;
impl pallet_election_provider_multi_phase::BenchmarkingConfig for ElectionProviderBenchmarkConfig {
	const VOTERS: [u32; 2] = [1000, 2000];
	const TARGETS: [u32; 2] = [500, 1000];
	const ACTIVE_VOTERS: [u32; 2] = [500, 800];
	const DESIRED_TARGETS: [u32; 2] = [200, 400];
	const SNAPSHOT_MAXIMUM_VOTERS: u32 = 1000;
	const MINER_MAXIMUM_VOTERS: u32 = 1000;
	const MAXIMUM_TARGETS: u32 = 300;
}

parameter_types! {
	pub const MaxCodeLen: u32 = 512 * 1024;
	pub const MaxStorageKeyLen: u32 = 128;
	pub Schedule: pallet_contracts::Schedule<Runtime> = pallet_contracts::Schedule::default();
	pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);

}

pub struct DummyRandomness;
impl frame_support::traits::Randomness<H256, BlockNumber> for DummyRandomness {
    fn random(_subject: &[u8]) -> (H256, BlockNumber) {
        (Default::default(), 0)
    }
}

impl pallet_contracts::Config for Runtime{
	type Time = Timestamp;
	type Randomness = DummyRandomness;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type RuntimeHoldReason = RuntimeHoldReason;
	type CallFilter = Nothing;
	type WeightPrice = TransactionPayment;
	type WeightInfo = pallet_contracts::weights::SubstrateWeight<Runtime>;
	type ChainExtension = (); 
	type Schedule = Schedule;
	type CallStack = [pallet_contracts::Frame<Self>; 5];

	type DepositPerByte = DepositPerByte;
	type DepositPerItem = DepositPerItem;
	type DefaultDepositLimit = DefaultDepositLimit;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent; // 30%
	type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
	type MaxCodeLen = MaxCodeLen;
	type MaxStorageKeyLen = MaxStorageKeyLen;
	type MaxTransientStorageSize = ConstU32<{ 64 * 1024 }>;
	type MaxDelegateDependencies = ConstU32<32>;
	type UnsafeUnstableInterface = ConstBool<false>;
	type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;

	type UploadOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type InstantiateOrigin = frame_system::EnsureSigned<Self::AccountId>;
	type Migrations = ();
	type Debug = ();
	type Environment = ();
	type ApiVersion = ();
	type Xcm = (); 

}

impl pallet_election_provider_multi_phase::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EstimateCallFee = TransactionPayment;
	type SignedPhase = SignedPhase;
	type UnsignedPhase = ConstU32<0>;
	type BetterSignedThreshold = BetterSignedThreshold;
	type OffchainRepeat = OffchainRepeat;
	type MinerTxPriority = MinerTxPriority;
	type MinerConfig = Self;
	type SignedMaxSubmissions = ConstU32<10>;
	type SignedRewardBase = SignedRewardBase;
	type SignedDepositBase = SignedDepositBase;
	type SignedDepositByte = SignedDepositByte;
	type SignedMaxRefunds = ConstU32<3>;
	type SignedMaxWeight = SignedMaxWeight;
	type MaxWinners = MaxActiveValidators;
	type SignedDepositWeight = ();
	type ElectionBounds = ElectionBoundsWrapper;
	type SlashHandler = ();
	type RewardHandler = ();
	type DataProvider = Staking;
	type Fallback =  onchain::OnChainExecution<OnChainSeqPhragmen>;
	type GovernanceFallback = onchain::OnChainExecution<OnChainSeqPhragmen>;
	type Solver = SequentialPhragmen<AccountId, SolutionAccuracyOf<Self>, OffchainRandomBalancing>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type BenchmarkingConfig = ElectionProviderBenchmarkConfig;
	type WeightInfo = election_provider_multi_phase::weights::SubstrateWeight<Runtime>;

}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = FungibleAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
	type WeightInfo = pallet_transaction_payment::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

/// Configure the pallet-template in pallets/template.
impl pallet_template::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_template::weights::SubstrateWeight<Runtime>;
}

impl CreateTransactionBase<pallet_election_provider_multi_phase::Call<Runtime>> for Runtime {
	type Extrinsic = UncheckedExtrinsic;
	type RuntimeCall = RuntimeCall;
}

impl CreateInherent<pallet_election_provider_multi_phase::Call<Runtime>> for Runtime {
	fn create_inherent(call: RuntimeCall) -> UncheckedExtrinsic {
		UncheckedExtrinsic::new_bare(call)
	}
}