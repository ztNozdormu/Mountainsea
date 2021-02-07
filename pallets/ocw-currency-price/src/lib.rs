//! A demonstration of an offchain worker that sends onchain callbacks

#![cfg_attr(not(feature = "std"), no_std)]

// #[cfg(test)]
// mod tests;

use core::{convert::TryInto, fmt};
use frame_support::{
	debug, decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult,
};
use parity_scale_codec::{Decode, Encode};

use frame_system::{
	self as system, ensure_none, ensure_signed,
	offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
		SignedPayload, SigningTypes, Signer, SubmitTransaction,
	},
};
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
	RuntimeDebug,
	offchain as rt_offchain,
	offchain::{
		storage::StorageValueRef,
		storage_lock::{StorageLock, BlockAndTime},
	},
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity,
		ValidTransaction,
	},
};
use sp_std::{
	prelude::*, str,
	collections::vec_deque::VecDeque,
};
use pallet_timestamp as timestamp;
use serde::{Deserialize, Deserializer};

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When an offchain worker is signing transactions it's going to request keys from type
/// `KeyTypeId` via the keystore to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"toke");
pub const NUM_VEC_LEN: usize = 10;
/// The type to sign and send transactions.
pub const UNSIGNED_TXS_PRIORITY: u64 = 100;
// 存储长度限制
pub const TOKENINFO_VEC_LEN: usize = 10;
// We are fetching information from the btcusdt public API about TOKEN IFNO . 交易对参数为: token_name+usdt(对标币种默认usdt)  例如：btcusdt
pub const TOKEN_USDT: &str = "usdt";
pub const HTTP_REMOTE_REQUEST: &str = "http://141.164.45.97:8080/ares/api/getPartyPrice/";
pub const HTTP_HEADER_USER_AGENT: &str = "Nozdormu";

pub const FETCH_TIMEOUT_PERIOD: u64 = 3000; // in milli-seconds
pub const LOCK_TIMEOUT_EXPIRATION: u64 = FETCH_TIMEOUT_PERIOD + 1000; // in milli-seconds
pub const LOCK_BLOCK_EXPIRATION: u32 = 3; // in block number

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrapper.
/// We can utilize the supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// them with the pallet-specific identifier.
pub mod crypto {
	use crate::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::app_crypto::{app_crypto, sr25519};
	use sp_runtime::{
		traits::Verify,
		MultiSignature, MultiSigner,
	};

	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;
	// implemented for ocw-runtime
	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Payload<Public> {
	number: u32,
	public: Public
}

impl <T: SigningTypes> SignedPayload<T> for Payload<T::Public> {
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

/// This is the pallet's configuration trait  离线工作机需要发起签名的交易，所以需要实现 CreateSignedTransaction 
pub trait Trait: system::Trait + CreateSignedTransaction<Call<Self>> + timestamp::Trait {
	/// The identifier type for an offchain worker.
	type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	/// The overarching dispatch call type.
	type Call: From<Call<Self>>;
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// ref: https://serde.rs/container-attrs.html#crate
#[derive(Deserialize, Encode, Decode, Default)]
struct TokenInfo {
	// Specify our own deserializing function to convert JSON string to vector of bytes
	#[serde(deserialize_with = "de_string_to_bytes")]
	name: Vec<u8>,
	#[serde(deserialize_with = "de_string_to_bytes")]
    exchange: Vec<u8>,
    price: u32,
    // timestamp:Timestamp,
}

pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
where
	D: Deserializer<'de>,
{
	let s: &str = Deserialize::deserialize(de)?;
	Ok(s.as_bytes().to_vec())
}

impl fmt::Debug for TokenInfo {
	// `fmt` converts the vector of bytes inside the struct back to string for
	//   more friendly display.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{{ login: {}, blog: {}, public_repos: {} }}",
			str::from_utf8(&self.name).map_err(|_| fmt::Error)?,
			str::from_utf8(&self.exchange).map_err(|_| fmt::Error)?,
			&self.price
		)
	}
}


decl_storage! {
	trait Store for Module<T: Trait> as Example {
		/// A vector of recently submitted numbers. Bounded by NUM_VEC_LEN
        Numbers get(fn numbers): VecDeque<u32>;
        // token 名称hash作为key : token一定时间内的信息作为value
        pub TokenInfos get(fn token_Infos): map hasher(blake2_128_concat) Vec<u8> => VecDeque<Option<TokenInfo>>;
        // 查询的token列表
        pub TokenNameList get(fn token_name_deque):  VecDeque<Option<Vec<u8>>>;
	}
}

decl_event!(
	/// Events generated by the module.
	pub enum Event<T>
	where
		AccountId = <T as system::Trait>::AccountId,
	{
		/// Event generated when a new token is accepted to contribute to the average.
        NewTokenName(Option<AccountId>, Vec<u8>),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
		// Error returned when not sure which ocw function to executed
		UnknownOffchainMux,

		// Error returned when making signed transactions in off-chain worker
        NoLocalAcctForSigning,
        
		OffchainSignedTxError,

		// Error returned when making unsigned transactions in off-chain worker
		OffchainUnsignedTxError,

		// Error returned when making unsigned transactions with signed payloads in off-chain worker
		OffchainUnsignedTxSignedPayloadError,

		// Error returned when fetching github info
		HttpFetchingError,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		#[weight = 10000]
		pub fn submit_token_name_signed(origin, token_name: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			debug::info!("submit_token_name_signed: ({:?}, {:?})", token_name, who);
			Self::append_or_replace_token_name(token_name);

			Self::deposit_event(RawEvent::NewTokenName(Some(who), token_name));
			Ok(())
		}

		#[weight = 0]
		pub fn update_token_info(origin, token_info: TokenInfo){
			let sender = ensure_signed(origin)?;
			//let token_deques = TokenInfos::get(Self::getLastTokenName());
			TokenInfos::get(Self::getLastTokenName())::mutate(|token_infos| {
				if token_infos.len() == TOKENINFO_VEC_LEN {
					let _ = token_infos.pop_front();
				}
				token_infos.push_back(Some(token_info.clone()));
       });
	
			Self::deposit_event(RawEvent::NewTokenName(Some(sender), token_info));
		}

		fn offchain_worker(block_number: T::BlockNumber) {
			debug::info!("Entering off-chain worker");

			// Here we are showcasing various techniques used when running off-chain workers (ocw)
			// 1. Sending signed transaction from ocw
			// 2. Sending unsigned transaction from ocw
			// 3. Sending unsigned transactions with signed payloads from ocw
			// 4. Fetching JSON via http requests in ocw
			const TX_TYPES: u32 = 4;
			let modu = block_number.try_into().map_or(TX_TYPES, |bn: u32| bn % TX_TYPES);
			let result = match modu {
				// 0 => Self::offchain_signed_tx(block_number),
				// 1 => Self::offchain_unsigned_tx(block_number),
				// 2 => Self::offchain_unsigned_tx_signed_payload(block_number),
				0 => Self::offchain_collect_signed_token_info(),
				_ => Err(Error::<T>::UnknownOffchainMux),
			};

			if let Err(e) = result {
				debug::error!("offchain_worker error: {:?}", e);
			}
		}
	}
}

impl<T: Trait> Module<T> {
	/// Append a new number to the tail of the list, removing an element from the head if reaching
	///   the bounded length.
	fn append_or_replace_token_name(token_name: Vec<u8>) {
		TokenNameList::mutate(|token_names| {
			if token_names.len() == NUM_VEC_LEN {
				let _ = token_names.pop_front();
			}
			token_names.push_back(Some(token_name));
			debug::info!("Number vector: {:?}", token_names);
		});
	}

	/// Check if we have fetched github info before. If yes, we can use the cached version
	///   stored in off-chain worker storage `storage`. If not, we fetch the remote info and
	///   write the info into the storage for future retrieval.
	fn fetch_token_info(token_name: Vec<u8>) -> Result<(), Error<T>> {
		// Create a reference to Local Storage value.
		// Since the local storage is common for all offchain workers, it's a good practice
		// to prepend our entry with the pallet name.
		let token_infos = StorageValueRef::persistent(b"ocw-currency-price::token-info");

		// Local storage is persisted and shared between runs of the offchain workers,
		// offchain workers may run concurrently. We can use the `mutate` function to
		// write a storage entry in an atomic fashion.
		//
		// With a similar API as `StorageValue` with the variables `get`, `set`, `mutate`.
		// We will likely want to use `mutate` to access
		// the storage comprehensively.
		//
		// Ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/storage/struct.StorageValueRef.html
		if let Some(Some(token_info)) = token_infos.get::<TokenInfo>() {
			// gh-info has already been fetched. Return early.
			debug::info!("cached token-info: {:?}", token_info);
			return Ok(());
		}

		// Since off-chain storage can be accessed by off-chain workers from multiple runs, it is important to lock
		//   it before doing heavy computations or write operations.
		// ref: https://substrate.dev/rustdocs/v2.0.0-rc3/sp_runtime/offchain/storage_lock/index.html
		//
		// There are four ways of defining a lock:
		//   1) `new` - lock with default time and block exipration
		//   2) `with_deadline` - lock with default block but custom time expiration
		//   3) `with_block_deadline` - lock with default time but custom block expiration
		//   4) `with_block_and_time_deadline` - lock with custom time and block expiration
		// Here we choose the most custom one for demonstration purpose.
		let mut lock = StorageLock::<BlockAndTime<Self>>::with_block_and_time_deadline(
			b"offchain-demo::lock", LOCK_BLOCK_EXPIRATION,
			rt_offchain::Duration::from_millis(LOCK_TIMEOUT_EXPIRATION)
		);

		// We try to acquire the lock here. If failed, we know the `fetch_n_parse` part inside is being
		//   executed by previous run of ocw, so the function just returns.
		// ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/storage_lock/struct.StorageLock.html#method.try_lock
		if let Ok(_guard) = lock.try_lock() {
			match Self::fetch_n_parse() {
				Ok(token_info) => { token_infos.set(&token_info); }
				Err(err) => { return Err(err); }
			}
		}
		Ok(())
	}

	/// Fetch from remote and deserialize the JSON to a struct
	fn fetch_n_parse() -> Result<TokenInfo, Error<T>> {
		let resp_bytes = Self::fetch_from_remote().map_err(|e| {
			debug::error!("fetch_from_remote error: {:?}", e);
			<Error<T>>::HttpFetchingError
		})?;

		let resp_str = str::from_utf8(&resp_bytes).map_err(|_| <Error<T>>::HttpFetchingError)?;
		// Print out our fetched JSON string
		debug::info!("{}", resp_str);

		// Deserializing JSON to struct, thanks to `serde` and `serde_derive`
		let token_info: TokenInfo =
			serde_json::from_str(&resp_str).map_err(|_| <Error<T>>::HttpFetchingError)?;
		Ok(token_info)
	}

	/// This function uses the `offchain::http` API to query the remote github information,
	///   and returns the JSON response as vector of bytes.
	fn fetch_from_remote() -> Result<Vec<u8>, Error<T>> {
		// 取TokenNameList最新的代币
		let token_name = String::from_utf8(Self::getLastTokenName()).expect("Found invalid UTF-8");
		let request_url = format!("{}{}{}", HTTP_REMOTE_REQUEST, token_name, TOKEN_USDT);
		debug::error!("fetch_from_remote URL: {:?}", request_url);
		debug::info!("sending request to: {}", request_url);
		
        
		// Initiate an external HTTP GET request. This is using high-level wrappers from `sp_runtime`.
		let request = rt_offchain::http::Request::get(HTTP_REMOTE_REQUEST);

		// Keeping the offchain worker execution time reasonable, so limiting the call to be within 3s.
		let timeout = sp_io::offchain::timestamp()
			.add(rt_offchain::Duration::from_millis(FETCH_TIMEOUT_PERIOD));

		// For github API request, we also need to specify `user-agent` in http request header.
		//   See: https://developer.github.com/v3/#user-agent-required
		let pending = request
			.add_header("User-Agent", HTTP_HEADER_USER_AGENT)
			.deadline(timeout) // Setting the timeout time
			.send() // Sending the request out by the host
			.map_err(|_| <Error<T>>::HttpFetchingError)?;

		// By default, the http request is async from the runtime perspective. So we are asking the
		//   runtime to wait here.
		// The returning value here is a `Result` of `Result`, so we are unwrapping it twice by two `?`
		//   ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait
		let response = pending
			.try_wait(timeout)
			.map_err(|_| <Error<T>>::HttpFetchingError)?
			.map_err(|_| <Error<T>>::HttpFetchingError)?;

		if response.code != 200 {
			debug::error!("Unexpected http request status code: {}", response.code);
			return Err(<Error<T>>::HttpFetchingError);
		}

		// Next we fully read the response body and collect it to a vector of bytes.
		Ok(response.body().collect::<Vec<u8>>())
	}
	// 获取最后一个代币名称
	fn getLastTokenName() -> Vec<u8> {
		//let token_name = 
		//Ok(token_name)
		vec![11,22,33]
		// The case of `None`: no token name is available for search
	//	debug::error!("No token name available");
		//Err(<Error<T>>::NoLocalAcctForSigning)
	}
	
    // 签名交易方法 处理最新的token信息
	fn offchain_collect_signed_token_info() -> Result<(), Error<T>> {
		// We retrieve a signer and check if it is valid.
		//   Since this pallet only has one key in the keystore. We use `any_account()1 to
		//   retrieve it. If there are multiple keys and we want to pinpoint it, `with_filter()` can be chained,
        //   ref: https://substrate.dev/rustdocs/v2.0.0/frame_system/offchain/struct.Signer.html
        // 提取签名账号
		let signer = Signer::<T, T::AuthorityId>::any_account();
		// 查询代币信息
		let tokenInfo: TokenInfo = Self::fetch_n_parse().map_err(|_| <Error<T>>::HttpFetchingError)?;
		// TODO
		//let token_deques: VecDeque<Option<Vec<TokenInfo>>> = TokenInfos::get(Self::getLastTokenName());
		//let mut token_deques = VecDeque::<Vec<TokenInfo>>::new();//VecDeque::<Option<Vec<TokenInfo>>::new(); : VecDeque<Vec<TokenInfo>>
		//let mut token_deque:  VecDeque<Vec<u8>> = VecDeque::new();
			// token_deque.push_back(vec![tokenInfo]); 
		//	token_deque::mutate(|tokenInfos| { 
		//		if tokenInfos.len() == TOKENINFO_VEC_LEN {
		//			let _ = tokenInfos.pop_front();
		//		}
		//		tokenInfos.push_back(Some(vec![tokenInfo]));
		//	});
		  // TokenInfos::insert(Self::getLastTokenName(),token_deque);
		// Translating the last token name and submit it on-chain TokenNameList::<T>::get(TokenNameList.len());
        let token_name_list =  TokenNameList::get();
        let token_name  = token_name_list.get(token_name_list.len()-1);

		// `result` is in the type of `Option<(Account<T>, Result<(), ()>)>`. It is:
		//   - `None`: no account is available for sending transaction
		//   - `Some((account, Ok(())))`: transaction is successfully sent
		//   - `Some((account, Err(())))`: error occured when sending the transaction
		let result = signer.send_signed_transaction(|_acct|
            // This is the on-chain function
            //Call::submit_token_name_signed(token_name.unwrap()),
            match token_name {
                Some(token_name)   => Call::submit_token_name_signed(token_name.unwrap()),
                None          => println!("No token_name? Oh well."),
            }
		);

		// Display error if the signed tx fails.
		if let Some((acc, res)) = result {
			if res.is_err() {
				debug::error!("failure: offchain_signed_tx: tx sent: {:?}", acc.id);
				return Err(<Error<T>>::OffchainSignedTxError);
			}
            // Transaction is sent successfully TODO 维护token信息
            
			return Ok(());
		}

		// The case of `None`: no account is available for sending
		debug::error!("No local account available");
		Err(<Error<T>>::NoLocalAcctForSigning)
	}
	
}

impl<T: Trait> rt_offchain::storage_lock::BlockNumberProvider for Module<T> {
	type BlockNumber = T::BlockNumber;
	fn current_block_number() -> Self::BlockNumber {
	  <frame_system::Module<T>>::block_number()
	}
}
