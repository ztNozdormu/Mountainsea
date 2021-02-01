
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch, traits::Get,
	dispatch::DispatchResult, ensure};
use frame_system::ensure_signed;
use codec::{Encode, Decode};

#[cfg(test)]
mod tests;

pub trait Trait: frame_system::Trait {
	/// Because this pallet emits events, it depends on the runtime's definition of an event.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

type TokenIndex = u64;
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Erc20 {
	name: Vec<u8>,
	ticker: Vec<u8>,
	total_supply: u128,
}

decl_storage! {
	// A unique name is used to ensure that the pallet's storage items are isolated.
	// This name may be updated, but each pallet in the runtime must use a unique name.
	// ---------------------------------vvvvvvvvvvvvvv
	trait Store for Module<T: Trait> as TemplateModule {
		// Learn more about declaring storage items:
		// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
        Balance get(fn balance_of): map hasher(blake2_128_concat) (TokenIndex, T::AccountId) => u128;
		Tokens get(fn tokens): map hasher(blake2_128_concat) TokenIndex => Option<Erc20>;
		TokenId get(fn token_id): TokenIndex;
		Allowance get(fn allowance): map hasher(blake2_128_concat) (TokenIndex, T::AccountId, T::AccountId) => u128;
    }
}

decl_event! {
	pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId, {
		Transfer(AccountId, AccountId, TokenIndex, u128),
		NewToken(AccountId, TokenIndex, u128),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Attempted to initialize the token after it had already been initialized.
		AlreadyInitialized,
		/// Attempted to transfer more funds than were available
		InsufficientFunds,
		TokenNotExists,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		#[weight = 10_000]
		fn init(origin, name: Vec<u8>, ticker: Vec<u8>, total_supply: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			<Balance<T>>::insert((Self::token_id(), sender), total_supply);
			Tokens::insert(Self::token_id(), Erc20{
				name: name,
				ticker: ticker,
				total_supply: total_supply,
			});
			TokenId::set(Self::token_id() + 1);
			Ok(())

		}

		#[weight = 10_000]
		fn transfer(origin, token_id: TokenIndex, to_account: T::AccountId, balance: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Tokens::contains_key(token_id), Error::<T>::TokenNotExists);
			ensure!(Self::balance_of((token_id, sender.clone())) >= balance, Error::<T>::InsufficientFunds);

			
			<Balance<T>>::mutate((token_id, sender.clone()), |value| *value = *value - balance);

			<Balance<T>>::mutate((token_id, to_account.clone()), |value| *value = *value + balance);

			Self::deposit_event(RawEvent::Transfer(sender, to_account, token_id, balance));

			Ok(())

		}
	}
}
