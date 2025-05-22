// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Genesis Presets for the Kitchensink Runtime

use crate::{
	constants::currency::*, AccountId,
	BabeConfig, GrandpaConfig, Balance, Signature, ImOnlineConfig, BalancesConfig, NominationPoolsConfig, SystemConfig,
	RuntimeGenesisConfig, SessionConfig, SessionKeys, StakingConfig,
	SudoConfig, BABE_GENESIS_EPOCH_CONFIG,
};
use frame_support::build_struct_json_patch;
use alloc::{vec, vec::Vec, format};
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use pallet_im_online::ed25519::AuthorityId as ImOnlineId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_genesis_builder::PresetId;
use sp_keyring::Sr25519Keyring;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};
use pallet_staking::StakerStatus;
use hex_literal::hex;
use sp_core::crypto::UncheckedFrom;
pub const ENDOWMENT: Balance = 10_000_000 * DOLLARS;
pub const STASH: Balance = ENDOWMENT / 1000;

pub struct StakingPlaygroundConfig {
	pub validator_count: u32,
	pub minimum_validator_count: u32,
}

/// The staker type as supplied ot the Staking config.
pub type Staker = (AccountId, AccountId, Balance, StakerStatus<AccountId>);

/// Helper function to create RuntimeGenesisConfig json patch for testing.
pub fn kitchensink_genesis(
	initial_authorities: Vec<(AccountId, AccountId, SessionKeys)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	stakers: Vec<Staker>,
	staking_playground_config: Option<StakingPlaygroundConfig>,
) -> serde_json::Value {
	let (validator_count, min_validator_count) = match staking_playground_config {
		Some(c) => (c.validator_count, c.minimum_validator_count),
		None => {
			let authorities_count = initial_authorities.len() as u32;
			(authorities_count, authorities_count)
		},
	};

	let collective = collective(&endowed_accounts);

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect(),
			..Default::default()
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|(stash, controller, keys_blob)| { 
					(
						stash.clone(),
						controller.clone(),
						keys_blob.clone(), 
					)
				})
				.collect(),
		},
		babe: BabeConfig {
			authorities: vec![], 
			epoch_config: BABE_GENESIS_EPOCH_CONFIG,
			..Default::default()
	    },

		grandpa: Default::default(),
		
		staking: StakingConfig {
			validator_count,
			minimum_validator_count: min_validator_count,
			invulnerables: initial_authorities
				.iter()
				.map(|x| x.0.clone())
				.collect::<Vec<_>>()
				.try_into()
				.expect("Too many invulnerable validators: upper limit is MaxInvulnerables from pallet staking config"),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
		},

		im_online: ImOnlineConfig { keys: vec![] },
		nomination_pools: NominationPoolsConfig {
			min_create_bond: 10 * DOLLARS,
			min_join_bond: 1 * DOLLARS,
			..Default::default()
		},
		transaction_payment: Default::default(),
		sudo: SudoConfig {
			key: Some(root_key),
		},
	})
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	// Note: Can't use `Sr25519Keyring::Alice.to_seed()` because the seed comes with `//`.
	let (alice_stash, alice, alice_session_keys) = authority_keys_from_seed("Alice");
	let (bob_stash, _bob, bob_session_keys) = authority_keys_from_seed("Bob");

	let endowed = well_known_including_eth_accounts();

	let patch = match id.as_ref() {
		sp_genesis_builder::DEV_RUNTIME_PRESET => kitchensink_genesis(
			// Use stash as controller account, otherwise grandpa can't load the authority set at
			// genesis.
			vec![(alice_stash.clone(), alice_stash.clone(), alice_session_keys)],
			alice.clone(),
			endowed,
			vec![validator(alice_stash.clone())],
			None,
		),
		sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => kitchensink_genesis(
			vec![
				// Use stash as controller account, otherwise grandpa can't load the authority set
				// at genesis.
				(alice_stash.clone(), alice_stash.clone(), alice_session_keys),
				(bob_stash.clone(), bob_stash.clone(), bob_session_keys),
			],
			alice,
			endowed,
			vec![validator(alice_stash), validator(bob_stash)],
			None,
		),
		"staging" => {
			let initial_authorities: Vec<(AccountId, AccountId, SessionKeys)> = vec![
				(
					// stash
					sp_core::sr25519::Public::unchecked_from(hex!("d41e0bf1d76de368bdb91896b0d02d758950969ea795b1e7154343ee210de649")).into(),
					// controller
					sp_core::sr25519::Public::unchecked_from(hex!("382bd29103cf3af5f7c032bbedccfb3144fe672ca2c606147974bc2984ca2b14")).into(),
					SessionKeys {
						babe: sp_consensus_babe::AuthorityId::unchecked_from(hex!("48640c12bc1b351cf4b051ac1cf7b5740765d02e34989d0a9dd935ce054ebb21")),
						grandpa: sp_consensus_grandpa::AuthorityId::unchecked_from(hex!("01a474a93a0cf830fb40b1d17fd1fc7c6b4a95fa11f90345558574a72da0d4b1")),
						im_online: pallet_im_online::ed25519::AuthorityId::unchecked_from(hex!("50041e469c63c994374a2829b0b0829213abd53be5113e751043318a9d7c0757")),
					},
				),
				(
					sp_core::sr25519::Public::unchecked_from(hex!("08050f1b6bcd4651004df427c884073652bafd54e5ca25cea69169532db2910b")).into(),
					sp_core::sr25519::Public::unchecked_from(hex!("8275157f2a1d8373106cb00078a73a92a3303f3bf6eb72c3a67413bd943b020b")).into(),
					SessionKeys {
						babe: sp_consensus_babe::AuthorityId::unchecked_from(hex!("0ecddcf7643a98de200b80fe7b18ebd38987fa106c5ed84fc004fa75ea4bac67")),
						grandpa: sp_consensus_grandpa::AuthorityId::unchecked_from(hex!("acdfcce0e40406fac1a8198c623ec42ea13fc627e0274bbb6c21e0811482ce13")),
						im_online: pallet_im_online::ed25519::AuthorityId::unchecked_from(hex!("6ac58683d639d3992a0090ab15f8c1dcf5a5ab7652fc9de60845441f9fc93903")),
					},
				),
				(
					sp_core::sr25519::Public::unchecked_from(hex!("861c6d95051f942bb022f13fc2125b2974933d8ab1441bfdee9855e9d8051556")).into(),
					sp_core::sr25519::Public::unchecked_from(hex!("8801f479e09a78515f1badee0169864dae45648109091e29b03a7b4ea97ec018")).into(),
					SessionKeys {
						babe: sp_consensus_babe::AuthorityId::unchecked_from(hex!("0c4d9de1e313572750abe19140db56433d20e4668e09de4df81a36566a8f2528")),
						grandpa: sp_consensus_grandpa::AuthorityId::unchecked_from(hex!("e493d74f9fa7568cca9dd294c9619a54c2e1b6bd3ecf3677fa7f9076b98c3fcd")),
						im_online: pallet_im_online::ed25519::AuthorityId::unchecked_from(hex!("c2e2a133b23995a48ff46cc704ef61929ee4a29b5fa468e41019ac63f3694e1f")),
					},
				),
				(
					sp_core::sr25519::Public::unchecked_from(hex!("ac8fdba5bbe008f65d0e85181daa5443c2eb492fea729a5981b2161467f8655c")).into(),
					sp_core::sr25519::Public::unchecked_from(hex!("ac039bef73f76755d3747d711554f7fb0f16022da51483e0d600c9c7c8cbf821")).into(),
					SessionKeys {
						babe: sp_consensus_babe::AuthorityId::unchecked_from(hex!("ca2245b6fa117fab9353a2031104d1d5d62e311957f375762324e65d71127465")),
						grandpa: sp_consensus_grandpa::AuthorityId::unchecked_from(hex!("392c51bf0c08f89cb1e091782d81359475d780986968ba7f6fa60f41feda6bf7")),
						im_online: pallet_im_online::ed25519::AuthorityId::unchecked_from(hex!("e68c9a2ee25e1999a4e87906aea429f3e5f3fc8dc9cd89f423d82860c6937b2e")),
					},
				),
			];

			let root_key: AccountId = sp_core::sr25519::Public::unchecked_from(hex!("9eaf896d76b55e04616ff1e1dce7fc5e4a417967c17264728b3fd8fee3b12f3c")).into();

			// Endow all stash and controller accounts, plus root_key
			let mut endowed_accounts: Vec<AccountId> = initial_authorities
				.iter()
				.flat_map(|(stash, controller, _)| vec![stash.clone(), controller.clone()])
				.collect();
			endowed_accounts.push(root_key.clone());

			// Remove duplicates
			endowed_accounts.sort();
			endowed_accounts.dedup();

			kitchensink_genesis(
				initial_authorities.clone(),
				root_key,
				endowed_accounts,
				initial_authorities.iter().map(|x| validator(x.0.clone())).collect(),
				None,
			)
		}
		,
		_ => return None,
	};

	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
	vec![
		PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
	]
}

/// Sets up the `account` to be a staker of validator variant as supplied to the
/// staking config.
pub fn validator(account: AccountId) -> Staker {
	// validator, controller, stash, staker status
	(account.clone(), account, STASH, StakerStatus::Validator)
}

/// Extract some accounts from endowed to be put into the collective.
fn collective(endowed: &[AccountId]) -> Vec<AccountId> {
	const MAX_COLLECTIVE_SIZE: usize = 50;
	let endowed_accounts_count = endowed.len();
	endowed
		.iter()
		.take(((endowed_accounts_count + 1) / 2).min(MAX_COLLECTIVE_SIZE))
		.cloned()
		.collect()
}

/// The Keyring's wellknown accounts + Alith and Baltathar.
///
/// Some integration tests require these ETH accounts.
pub fn well_known_including_eth_accounts() -> Vec<AccountId> {
	Sr25519Keyring::well_known()
		.map(|k| k.to_account_id())
		.chain([
			// subxt_signer::eth::dev::alith()
			array_bytes::hex_n_into_unchecked(
				"f24ff3a9cf04c71dbc94d0b566f7a27b94566caceeeeeeeeeeeeeeeeeeeeeeee",
			),
			// subxt_signer::eth::dev::baltathar()
			array_bytes::hex_n_into_unchecked(
				"3cd0a705a2dc65e5b1e1205896baa2be8a07c6e0eeeeeeeeeeeeeeeeeeeeeeee",
			),
		])
		.collect::<Vec<_>>()
}

// The URL for the telemetry server.
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed.
///
/// Note: `//` is prepended internally.
pub fn authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, SessionKeys) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        session_keys_from_seed(seed),
    )
}

pub fn session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
) -> SessionKeys {
	SessionKeys { grandpa, babe, im_online }
}

/// We have this method as there is no straight forward way to convert the
/// account keyring into these ids.
///
/// Note: `//` is prepended internally.
pub fn session_keys_from_seed(seed: &str) -> SessionKeys {
	session_keys(
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),      // Add BABE key
        get_from_seed::<ImOnlineId>(seed),
	)
}

// Genesis builder for testnet/dev presets.
pub fn testnet_genesis_preset(
	initial_authorities: Vec<(AccountId, AccountId, SessionKeys)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
) -> serde_json::Value {
	use frame_support::build_struct_json_patch;
	use sp_runtime::Perbill;
	use pallet_staking::StakerStatus;

	let stakers = initial_authorities
		.iter()
		.map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
		.collect::<Vec<_>>();

	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect(),
			..Default::default()
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|(stash, controller, keys_blob)| { 
					(
						stash.clone(),
						controller.clone(),
						keys_blob.clone(), 
					)
				})
				.collect(),
		},
		babe: BabeConfig {
			authorities: vec![], 
			epoch_config: BABE_GENESIS_EPOCH_CONFIG,
			..Default::default()
	    },
		grandpa: Default::default(),
		staking: StakingConfig {
			validator_count: initial_authorities.len() as u32,
			minimum_validator_count: initial_authorities.len() as u32,
			invulnerables: initial_authorities
				.iter()
				.map(|x| x.0.clone())
				.collect::<Vec<_>>()
				.try_into()
				.expect("Too many invulnerable validators: upper limit is MaxInvulnerables from pallet staking config"),
			slash_reward_fraction: Perbill::from_percent(10),
			stakers,
			..Default::default()
		},
		im_online: ImOnlineConfig { keys: vec![] },
		sudo: SudoConfig {
			key: Some(root_key),
		},
		nomination_pools: NominationPoolsConfig {
			min_create_bond: 10 * DOLLARS,
			min_join_bond: 1 * DOLLARS,
			..Default::default()
		},
		transaction_payment: Default::default(),
		// Add any other pallets as needed, e.g. assets, etc.
	})
}