// node/src/chain_spec.rs

use sc_service::ChainType;
use solo_substrate_runtime::WASM_BINARY;

pub type ChainSpec = sc_service::GenericChainSpec;

pub fn development_chain_spec() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Development")
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_preset_name(sp_genesis_builder::DEV_RUNTIME_PRESET)
	.build())
}

pub fn local_chain_spec() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.build())
}

pub fn staging_network_config() -> ChainSpec {
	ChainSpec::builder(
		WASM_BINARY.expect("WASM binary was not built, please build the runtime first"),
		None,
	)
	.with_name("Substrate Stencil")
	.with_id("stencil_network")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_preset_name("staging")
	// Optionally add telemetry, protocol id, properties, extensions, bootnodes, etc.
	.build()
}

// If you want to support a staging/live network, add a similar function here
// referencing a new preset name, and add the preset logic in genesis_config_presets.rs.






