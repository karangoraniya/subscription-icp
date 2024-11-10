use ic_cdk_macros::{init, update};
use std::cell::RefCell;
use std::time::Duration;

use alloy::{
    network::EthereumWallet,
    primitives::{address, U256},
    providers::{Provider, ProviderBuilder},
    signers::Signer,
    sol,
    transports::icp::IcpConfig,
};

use crate::{create_icp_signer, get_rpc_service_sepolia};

thread_local! {
    static NONCE: RefCell<Option<u64>> = const { RefCell::new(None) };
}

// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(missing_docs, clippy::too_many_arguments)]
    #[sol(rpc)]
    USDC,
    "abi/USDC.json"
);

/// This function will attempt to transfer a small amount of USDC to the ethereum address of the canister.
///
/// Nonce handling is implemented manually instead of relying on the Alloy built in
/// `with_recommended_fillers` method. This minimizes the number of requests sent to the
/// EVM RPC.
///
/// The following RPC calls are made to complete a transaction:
/// - `eth_getTransactionCount`: To determine the next nonce. This call is only made once after
/// canister deployment, then the nonces are cached.
/// - `eth_estimateGas`: To determine the gas limit
/// - `eth_sendRawTransaction`: The transaction
/// - `eth_getTransactionByHash`: To determine if transaction was successful. Increment nonce only
/// if transaction was successful.
///
/// Even though this function makes half as many RPC calls as `send_eth_with_fillers` it is still
/// recommended to use a deduplication proxy between the EVM RPC canister and the RPC provider
/// (Alchemy, etc). For a fully decentralised deployment, one option is also to deploy a copy of
/// the EVM RPC canister yourself on an app subnet with only 13 nodes and your own RPC API key.
/// Perhaps 3 calls * 13 = 39 fits within the RPC call limits.
///
///     
const N: Duration = Duration::from_secs(10);

#[init]
fn init() {
    ic_cdk_timers::set_timer_interval(N, || {
        ic_cdk::print("right before timer trap2");
        ic_cdk::spawn(async {
            if let Err(e) = transfer_usdc().await {
                ic_cdk::println!("Error in transfer_usdc: {:?}", e);
            } else {
                ic_cdk::println!("Success in transfer_usdc");
            }
        });
    });
}

#[update]
async fn transfer_usdc() -> Result<String, String> {
    ic_cdk::print("right before timer trap");
    // Setup signer
    let signer = create_icp_signer().await;
    let address = signer.address();

    // Setup provider
    let wallet = EthereumWallet::from(signer);
    let rpc_service = get_rpc_service_sepolia();
    let config = IcpConfig::new(rpc_service)
        .set_max_response_size(200_000)
        .set_call_cycles(60_000_000_000);
    let provider = ProviderBuilder::new()
        .with_gas_estimation()
        .wallet(wallet)
        .on_icp(config);

    // Attempt to get nonce from thread-local storage
    let maybe_nonce = NONCE.with_borrow(|maybe_nonce| {
        // If a nonce exists, the next nonce to use is latest nonce + 1
        maybe_nonce.map(|nonce| nonce + 1)
    });

    // If no nonce exists, get it from the provider
    let nonce = if let Some(nonce) = maybe_nonce {
        nonce
    } else {
        provider.get_transaction_count(address).await.unwrap_or(0)
    };

    let contract = USDC::new(
        address!("1c7D4B196Cb0C7B01d743Fbc6116a902379C7238"),
        provider.clone(),
    );

    let from_address = address!("E0B2A968Fc566bce543E9da6D3893FfE1170B833"); // from address
    let to_address = address!("55Eca4d519Ca2BdC60C8f886aB00B5281772E517"); // receiver address
    let value: U256 = U256::from(10000); //100000

    match contract
        .transferFrom(from_address, to_address, value)
        .nonce(nonce)
        .chain_id(11155111)
        //.from(from_address)
        .send()
        .await
    {
        Ok(builder) => {
            let node_hash = *builder.tx_hash();
            let tx_response = provider.get_transaction_by_hash(node_hash).await.unwrap();

            match tx_response {
                Some(tx) => {
                    // The transaction has been mined and included in a block, the nonce
                    // has been consumed. Save it to thread-local storage. Next transaction
                    // for this address will use a nonce that is = this nonce + 1
                    NONCE.with_borrow_mut(|nonce| {
                        *nonce = Some(tx.nonce);
                    });
                    Ok(format!("{:?}", tx))
                }
                None => Err("Could not get transaction.".to_string()),
            }
        }
        Err(e) => Err(format!("{:?}", e)),
    }
}
