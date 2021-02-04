#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit="256"]
/// A FRAME pallet proof of existence with necessary imports

use frame_support::{storage::{StorageDoubleMap,StorageMap},
	decl_module, decl_storage, decl_event, decl_error, dispatch, ensure,
	traits::{Get},
};
use frame_support::traits::{Currency, ExistenceRequirement};
use frame_system::{self as system, ensure_signed};
use sp_std::prelude::*;
use sp_runtime::traits::StaticLookup;
use codec::{Encode,Decode};
// use pallet_timestamp as timestamp;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

/// The pallet's configuration trait.
pub trait Trait: system::Trait + timestamp::Trait  {
	// Add other types and constants required to configure this pallet.

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	// 附加题答案
	type MaxClaimLength: Get<u32>;

	// 添加Currency类型
	type Currency: Currency<Self::AccountId>;
}
#[derive(Encode, Decode,Default,Clone,PartialEq,Eq)]  //Debug,Eq,
#[cfg_attr(feature = "std",derive(Debug))]
pub struct ClaimInfo<AccountId,BlockNumber,Timestamp> {
	account_id: AccountId,
	block_number: BlockNumber,
	create_time: Timestamp,
	comment: Vec<u8>,
	claim_hash: Vec<u8>,
}
// This pallet's storage items.
decl_storage! {
	// It is important to update your storage name so that your pallet's
	// storage items are isolated from other pallets.
	// ---------------------------------vvvvvvvvvvvvvv
	trait Store for Module<T: Trait> as PoeModule {
		Proofs get(fn proofs): map hasher(blake2_128_concat) Vec<u8> => (T::AccountId, T::BlockNumber);
		Prices get(fn price): map hasher(blake2_128_concat) Vec<u8> => BalanceOf<T>;
		// 双键映射MAP数据存储结构  key1:file文件哈希字符串 key2:用户账号 value:tuple(存证创建备注信息,创建时间,区块高度)
		//ClaimInfos get(fn claim_infos): double_map hasher(blake2_128_concat) Vec<u8>,hasher(blake2_128_concat) T::AccountId => (T::AccountId,T::Moment, T::BlockNumber,Vec<u8>,Vec<u8>);
		ClaimInfos get(fn claim_infos): double_map hasher(blake2_128_concat) T::AccountId,hasher(blake2_128_concat) Vec<u8> => (T::AccountId, T::BlockNumber,T::Moment,Vec<u8>,Vec<u8>);//
        ClaimInfoList get(fn claim_info_list): Vec<ClaimInfo<T::AccountId, T::BlockNumber,T::Moment>>;
	}
}

// The pallet's events
decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId,
	   Balance = BalanceOf<T>,
	  {
		ClaimCreated(AccountId, Vec<u8>),
		ClaimRevoked(AccountId, Vec<u8>),
		PriceSet(AccountId, Vec<u8>, Balance),
		BuyClaimOk(AccountId,Vec<u8>, Balance),
		SaveClaimInfoOk(AccountId, Vec<u8>,Vec<u8>),
	}
);

// The pallet's errors
decl_error! {
	pub enum Error for Module<T: Trait> {
		ProofAlreadyExist,
		ClaimNotExist,
		NotClaimOwner,
		ProofTooLong,
		CanNotBuyOwnClaim,
		BuyPriceCanNOtLessThanSellPrice,
	}
}

// The pallet's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing errors
		// this includes information about your errors in the node's metadata.
		// it is needed only if you are using errors in your pallet
		type Error = Error<T>;

		// Initializing events
		// this is needed only if you are using events in your pallet
		fn deposit_event() = default;

		#[weight = 0]
		pub fn create_claim(origin, claim: Vec<u8>) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(!Proofs::<T>::contains_key(&claim), Error::<T>::ProofAlreadyExist);

			// 附加题答案
			ensure!(T::MaxClaimLength::get() >= claim.len() as u32, Error::<T>::ProofTooLong);

			Proofs::<T>::insert(&claim, (sender.clone(), system::Module::<T>::block_number()));

            // 存证创建时设置默认价格为0
            let zero_price = 0 as u32;
			let price: BalanceOf<T> = zero_price.into();
			Prices::<T>::insert(&claim,&price);

			Self::deposit_event(RawEvent::ClaimCreated(sender, claim));

			Ok(())
		}

		#[weight = 0]
		pub fn revoke_claim(origin, claim: Vec<u8>) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(Proofs::<T>::contains_key(&claim), Error::<T>::ClaimNotExist);

			let (owner, _block_number) = Proofs::<T>::get(&claim);

			ensure!(owner == sender, Error::<T>::NotClaimOwner);

			Proofs::<T>::remove(&claim);

			Prices::<T>::remove(&claim);

			Self::deposit_event(RawEvent::ClaimRevoked(sender, claim));

			Ok(())
		}

		// 第二题答案
		#[weight = 0]
		pub fn transfer_claim(origin, claim: Vec<u8>, dest: <T::Lookup as StaticLookup>::Source) -> dispatch::DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(Proofs::<T>::contains_key(&claim), Error::<T>::ClaimNotExist);

			let (owner, _block_number) = Proofs::<T>::get(&claim);

			ensure!(owner == sender, Error::<T>::NotClaimOwner);

			let dest = T::Lookup::lookup(dest)?;

			Proofs::<T>::insert(&claim, (dest, system::Module::<T>::block_number()));

			Ok(())
		}

		#[weight = 0]
		pub fn set_price(origin, claim: Vec<u8>,price: BalanceOf<T>) -> dispatch::DispatchResult {
		    let sender = ensure_signed(origin)?;
			ensure!(Proofs::<T>::contains_key(&claim),Error::<T>::ClaimNotExist);
			let (s,_) = Proofs::<T>::get(&claim);
			ensure!(s == sender,Error::<T>::NotClaimOwner);
			Prices::<T>::insert(&claim,&price);

			Self::deposit_event(RawEvent::PriceSet(sender,claim,price));

			Ok(())
		}
		/**
		 * origin buy claim user
		 * seller sell claim user
		 * sellPrice
		 * buyPrice
		 */
		 #[weight = 0]
		 pub fn buy_claim(origin,claim: Vec<u8>,buy_price: BalanceOf<T>) -> dispatch::DispatchResult {
		     let buyer = ensure_signed(origin)?;
		 	 ensure!(Proofs::<T>::contains_key(&claim),Error::<T>::ClaimNotExist);

			 let(owner,_) = Proofs::<T>::get(&claim);
			 ensure!(buyer!=owner,Error::<T>::CanNotBuyOwnClaim);
			 let sell_pric_least =  Prices::<T>::get(&claim);

			 ensure!(buy_price > sell_pric_least, Error::<T>::BuyPriceCanNOtLessThanSellPrice);

		 	 T::Currency::transfer(&buyer,&owner,sell_pric_least,ExistenceRequirement::AllowDeath)?;

			 Proofs::<T>::insert(&claim, (&buyer, system::Module::<T>::block_number()));

			 Prices::<T>::insert(&claim,&sell_pric_least);

			 Self::deposit_event(RawEvent::BuyClaimOk(buyer,claim,sell_pric_least));

			 Ok(())
		 }
		  /**
		 * 保存用户的存证详细信息
		 *  origin 存证创建人
		 *  claim  存证哈希数据
		 *  comment 备注信息字符串
 		 *  
		 */
		#[weight = 0]
		pub fn save_claim_info(origin, claim: Vec<u8>,comment: Vec<u8>) -> dispatch::DispatchResult {

			let sender = ensure_signed(origin)?;

			ensure!(T::MaxClaimLength::get() >= claim.len() as u32, Error::<T>::ProofTooLong);
			// init
            // let block_number: BlockNumber = 0;
            // let owner: AccountId = &sender;
			// if claim is no save first  save claim  then save the info else only save info
			 let mut block_number: T::BlockNumber = system::Module::<T>::block_number();
             if !Proofs::<T>::contains_key(&claim) {

                  Proofs::<T>::insert(&claim, (sender.clone(),block_number));
                    // 存证创建时设置默认价格为0
                    let zero_price = 0 as u32;
					let price: BalanceOf<T> = zero_price.into();
					Prices::<T>::insert(&claim,&price);
             }else{
               let (owner,block_number_1) = Proofs::<T>::get(&claim);
                block_number = block_number_1;
			    ensure!(owner == sender, Error::<T>::NotClaimOwner);
             }
            // save claim info
			// let create_time = <timestamp::Module<T>>::get();
            // key1 claim hash key2 AccountId  value claimInfo
            // claim info object
            // let claim_info = ClaimInfo {
            //     owner: &sender,
            //     create_time: <timestamp::Module<T>>::get(),
            //     block_height: block_number,
            //     comment: comment,
            //     claim_hash: claim,
            // };
			// ClaimInfos::<T>::insert(&claim,&sender,claim_info);
			let create_time = <timestamp::Module<T>>::get();
			// ClaimInfos::<T>::insert(&sender,&claim,(&sender,create_time,block_number,&comment,&claim));
			ClaimInfos::<T>::insert(&sender,&claim,(&sender,block_number,create_time,&comment,&claim));

			Self::deposit_event(RawEvent::SaveClaimInfoOk(sender, claim,comment));

			Ok(())
		}

		#[weight = 0]
		pub fn claim_list_by_account(origin) -> dispatch::DispatchResult {
		let sender = ensure_signed(origin)?;
		  // clear history data
          <ClaimInfoList<T>>::kill();
          // iter double_map : Vec<(T::AccountId,T::Moment,T::BlockNumber,Vec<u8>,Vec<u8>)>
            let result: Vec<(T::AccountId,T::BlockNumber,T::Moment,Vec<u8>,Vec<u8>)> = ClaimInfos::<T>::iter_prefix_values(sender).collect::<Vec<(T::AccountId,T::BlockNumber,T::Moment,Vec<u8>,Vec<u8>)>>();

                 for (account_id,block_number,create_time,comment,claim_hash) in result {
                       let claim_info = ClaimInfo {
                                        account_id: account_id,
                                        block_number: block_number,
										create_time: create_time, //<timestamp::Module<T>>::get(),
										comment: comment,
										claim_hash: claim_hash,
									};
                        <ClaimInfoList<T>>::append(&claim_info);
                 }
		      Ok(())
		}
	}
}
