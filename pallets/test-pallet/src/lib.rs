#![cfg_attr(not(feature = "std"), no_std)]

/// A FRAME pallet template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references

/// For more guidance on Substrate FRAME, see the example pallet
/// https://github.com/paritytech/substrate/blob/master/frame/example/src/lib.rs

use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch, ensure, StorageMap};
use dispatch::{Parameter, DispatchResult};
use frame_system::{self as system, ensure_signed};
use sp_std::if_std;
use sp_runtime::traits::{Member, CheckedAdd, CheckedSub};
use codec::{Decode, Encode};
use sp_std::vec::Vec;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// The pallet's configuration trait.
pub trait Trait: system::Trait {
	// Add other types and constants required to configure this pallet.

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type TokenBalance: Parameter + Member + Default + Copy + From<u128> + CheckedAdd + CheckedSub + PartialEq + PartialOrd;
}

// struct to store the token details
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Erc20Token<U> {
    name: Vec<u8>,
    ticker: Vec<u8>,
    total_supply: U,
}


// storage for this module
decl_storage! {
	trait Store for Module<T: Trait> as Erc20 {
		// token id nonce for storing the next token id available for token initialization
		// inspired by the AssetId in the SRML assets module
		TokenId get(fn token_id): u32;
		// details of the token corresponding to a token id
		Tokens get(fn token_details): map hasher(blake2_128_concat) u32 => Erc20Token<T::TokenBalance>;
		// balances mapping for an account and token
		BalanceOf get(fn balance_of): map hasher(blake2_128_concat) (u32, T::AccountId) => T::TokenBalance;
		// allowance for an account and token
		Allowance get(fn allowance): map hasher(blake2_128_concat) (u32, T::AccountId, T::AccountId) => T::TokenBalance;
	}
  }
  
  // events
  decl_event!(
	  pub enum Event<T> where AccountId = <T as system::Trait>::AccountId, Balance = <T as self::Trait>::TokenBalance {
		  // event for transfer of tokens
		  // tokenid, from, to, value
		  Transfer(u32, AccountId, AccountId, Balance),
		  // event when an approval is made
		  // tokenid, owner, spender, value
		  Approval(u32, AccountId, AccountId, Balance),
	  }
  );


// The pallet's dispatchable functions.
decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		 // initialize the default event for this module
		 fn deposit_event() = default;

		 // initializes a new token
		 // generates an integer token_id so that all tokens are unique
		 // takes a name, ticker, total supply for the token
		 // makes the initiating account the owner of the token
		 // the balance of the owner is set to total supply
		 #[weight = 10_000]
		 fn init(origin, name: Vec<u8>, ticker: Vec<u8>, total_supply: T::TokenBalance) -> DispatchResult {
			 let sender = ensure_signed(origin)?;
			 
			 // checking max size for name and ticker
			 // byte arrays (vecs) with no max size should be avoided
			 ensure!(name.len() <= 64, "token name cannot exceed 64 bytes");
			 ensure!(ticker.len() <= 32, "token ticker cannot exceed 32 bytes");
   
			 let token_id = Self::token_id();

			 let next_token_id = token_id.checked_add(1).ok_or("overflow in calculating next token id")?;
			 <TokenId>::put(next_token_id);
   
			 let token = Erc20Token {
				 name,
				 ticker,
				 total_supply,
			 };
   
			 <Tokens<T>>::insert(token_id, token);
			 <BalanceOf<T>>::insert((token_id, sender), total_supply);
   
			 Ok(())
		 }
   
		 // transfer tokens from one account to another
		 // origin is assumed as sender
		 #[weight = 10_000]
		 fn transfer(_origin, token_id: u32, to: T::AccountId, value: T::TokenBalance) -> DispatchResult {
			 let sender = ensure_signed(_origin)?;
			 Self::_transfer(token_id, sender, to, value)
		 }
   
		 // approve token transfer from one account to another
		 // once this is done, transfer_from can be called with corresponding values
		 #[weight = 10_000]
		 fn approve(_origin, token_id: u32, spender: T::AccountId, value: T::TokenBalance) -> DispatchResult {
			 let sender = ensure_signed(_origin)?;
			 ensure!(<BalanceOf<T>>::contains_key((token_id, sender.clone())), "Account does not own this token");
   
			 let allowance = Self::allowance((token_id, sender.clone(), spender.clone()));
			 let updated_allowance = allowance.checked_add(&value).ok_or("overflow in calculating allowance")?;
			 <Allowance<T>>::insert((token_id, sender.clone(), spender.clone()), updated_allowance);
   
			 Self::deposit_event(RawEvent::Approval(token_id, sender.clone(), spender.clone(), value));
   
			 Ok(())
		 }
   
		 // the ERC20 standard transfer_from function
		 // implemented in the open-zeppelin way - increase/decrease allownace
		 // if approved, transfer from an account to another account without owner's signature
		 #[weight = 10_000]
		 pub fn transfer_from(_origin, token_id: u32, from: T::AccountId, to: T::AccountId, value: T::TokenBalance) -> DispatchResult {
		   ensure!(<Allowance<T>>::contains_key((token_id, from.clone(), to.clone())), "Allowance does not exist.");
		   let allowance = Self::allowance((token_id, from.clone(), to.clone()));
		   ensure!(allowance >= value, "Not enough allowance.");
			 
		   // using checked_sub (safe math) to avoid overflow
		   let updated_allowance = allowance.checked_sub(&value).ok_or("overflow in calculating allowance")?;
		   <Allowance<T>>::insert((token_id, from.clone(), to.clone()), updated_allowance);
   
		   Self::deposit_event(RawEvent::Approval(token_id, from.clone(), to.clone(), value));
		   Self::_transfer(token_id, from, to, value)
		 }
	 }
}

// implementation of mudule
// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
    // the ERC20 standard transfer function
    // internal
    fn _transfer(
        token_id: u32,
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
    ) -> DispatchResult {
        ensure!(<BalanceOf<T>>::contains_key((token_id, from.clone())), "Account does not own this token");
        let sender_balance = Self::balance_of((token_id, from.clone()));
        ensure!(sender_balance >= value, "Not enough balance.");

        let updated_from_balance = sender_balance.checked_sub(&value).ok_or("overflow in calculating balance")?;
        let receiver_balance = Self::balance_of((token_id, to.clone()));
        let updated_to_balance = receiver_balance.checked_add(&value).ok_or("overflow in calculating balance")?;
        
        // reduce sender's balance
        <BalanceOf<T>>::insert((token_id, from.clone()), updated_from_balance);

        // increase receiver's balance
        <BalanceOf<T>>::insert((token_id, to.clone()), updated_to_balance);

        Self::deposit_event(RawEvent::Transfer(token_id, from, to, value));
        Ok(())
    }
}


// The pallet's errors
decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Value was None
		NoneValue,
		/// Value reached maximum and cannot be incremented further
		StorageOverflow,
	}
}