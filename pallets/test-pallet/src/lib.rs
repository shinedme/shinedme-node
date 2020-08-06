#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use dispatch::{DispatchResult, Parameter};
/// A FRAME pallet template with necessary imports

/// Feel free to remove or edit this file as needed.
/// If you change the name of this file, make sure to update its references in runtime/src/lib.rs
/// If you remove this file, you can remove those references

/// For more guidance on Substrate FRAME, see the example pallet
/// https://github.com/paritytech/substrate/blob/master/frame/example/src/lib.rs
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch, ensure, StorageMap,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::traits::{CheckedAdd, CheckedSub, Member};
use sp_std::if_std;
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
    type TokenBalance: Parameter
        + Member
        + Default
        + Copy
        + From<u128>
        + CheckedAdd
        + CheckedSub
        + PartialEq
        + PartialOrd;
}

// struct to store the token details
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Erc20Token<U> {
    name: Vec<u8>,
    ticker: Vec<u8>,
    total_supply: U,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct AccountProfile {
    name: Vec<u8>,
    avatar: Vec<u8>,
    photos: Vec<Vec<u8>>
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct PhotoInfo<AccountId> {
    owner: AccountId,
    affiliate_url: Option<(AccountId, Vec<u8>)>,
    likes: Vec<AccountId>,
    variants: Vec<Vec<u8>>,
    comments: Vec<Vec<u8>>,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct AffiliateInfo<TokenBalance> {
    single_credit: TokenBalance,
    total_credit: TokenBalance,
}

// storage for this module
decl_storage! {
  trait Store for Module<T: Trait> as Erc20 {
      Initialized get(fn initialized): bool;
      Treasury get(fn treasury): T::AccountId;
      TokenInfo get(fn token_info): Erc20Token<T::TokenBalance>;
      // balances mapping for an account and token
      BalanceOf get(fn balance_of): map hasher(blake2_128_concat) T::AccountId => T::TokenBalance;
      // allowance for an account and token
      Allowance get(fn allowance): map hasher(blake2_128_concat) (T::AccountId, T::AccountId) => T::TokenBalance;
      Accounts get(fn accounts): map hasher(blake2_128_concat) T::AccountId => AccountProfile;
      Photos get(fn photos): map hasher(blake2_128_concat) Vec<u8> => PhotoInfo<T::AccountId>;
      Affiliations get(fn affiliations): map hasher(blake2_128_concat) (T::AccountId, Vec<u8>) => AffiliateInfo<T::TokenBalance>;
  }
}

// events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = <T as self::Trait>::TokenBalance,
    {
        // event for transfer of tokens
        // tokenid, from, to, value
        Transfer(AccountId, AccountId, Balance),
        // event when an approval is made
        // tokenid, owner, spender, value
        Approval(AccountId, AccountId, Balance),
        AccountUpdated(AccountId, Vec<u8>, Vec<u8>),
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
            ensure!(!Self::initialized(), "already initialized");
            ensure!(name.len() <= 64, "token name cannot exceed 64 bytes");
            ensure!(ticker.len() <= 32, "token ticker cannot exceed 32 bytes");

            let token = Erc20Token {
                name,
                ticker,
                total_supply,
            };
            <Initialized>::put(true);
            <Treasury<T>>::put(sender.clone());
            <TokenInfo<T>>::put(token);
            <BalanceOf<T>>::insert(sender, total_supply);

            Ok(())
        }

        // transfer tokens from one account to another
        // origin is assumed as sender
        #[weight = 10_000]
        fn transfer(_origin, to: T::AccountId, value: T::TokenBalance) -> DispatchResult {
            let sender = ensure_signed(_origin)?;
            Self::_transfer(sender, to, value)
        }

        // approve token transfer from one account to another
        // once this is done, transfer_from can be called with corresponding values
        #[weight = 10_000]
        fn approve(_origin, spender: T::AccountId, value: T::TokenBalance) -> DispatchResult {
            let sender = ensure_signed(_origin)?;
            ensure!(<BalanceOf<T>>::contains_key(sender.clone()), "Account does not own this token");

            let allowance = Self::allowance((sender.clone(), spender.clone()));
            let updated_allowance = allowance.checked_add(&value).ok_or("overflow in calculating allowance")?;
            <Allowance<T>>::insert((sender.clone(), spender.clone()), updated_allowance);

            Self::deposit_event(RawEvent::Approval(sender.clone(), spender.clone(), value));

            Ok(())
        }

        // the ERC20 standard transfer_from function
        // implemented in the open-zeppelin way - increase/decrease allownace
        // if approved, transfer from an account to another account without owner's signature
        #[weight = 10_000]
        pub fn transfer_from(_origin, from: T::AccountId, to: T::AccountId, value: T::TokenBalance) -> DispatchResult {
            ensure!(<Allowance<T>>::contains_key((from.clone(), to.clone())), "Allowance does not exist.");
            let allowance = Self::allowance((from.clone(), to.clone()));
            ensure!(allowance >= value, "Not enough allowance.");

            // using checked_sub (safe math) to avoid overflow
            let updated_allowance = allowance.checked_sub(&value).ok_or("overflow in calculating allowance")?;
            <Allowance<T>>::insert((from.clone(), to.clone()), updated_allowance);

            Self::deposit_event(RawEvent::Approval(from.clone(), to.clone(), value));
            Self::_transfer(from, to, value)
        }

        #[weight = 10_000]
        pub fn update_user(_origin, name: Vec<u8>, avatar: Vec<u8>) -> DispatchResult {
            let sender = ensure_signed(_origin)?;

            <Accounts<T>>::insert(sender, AccountProfile {name, avatar, photos: Vec::new()});
            Ok(())
        }

        #[weight = 10_000]
        pub fn upload_photo(_origin, photo: Vec<u8>, affiliate_url: Option<(T::AccountId, Vec<u8>)>) -> DispatchResult {
            let sender = ensure_signed(_origin)?;

            ensure!(!<Photos<T>>::contains_key(photo.clone()), "This photo already uploaded");
            // TODO: off chain verify this is actually exist in ipfs, and it's a photo.

            if let Some(affiliate_url) = affiliate_url {
                ensure!(<Affiliations<T>>::contains_key(affiliate_url.clone()), "Affiliation doesn't exist");
                <Photos<T>>::insert(photo.clone(), PhotoInfo { owner: sender.clone(), likes: Vec::new(), variants: Vec::new(), affiliate_url: Some(affiliate_url), comments: Vec::new()});
            } else {
                <Photos<T>>::insert(photo.clone(), PhotoInfo { owner: sender.clone(), likes: Vec::new(), variants: Vec::new(), affiliate_url: None, comments: Vec::new() });
            }
            let mut account = Self::accounts(sender.clone());
            account.photos.push(photo);
            <Accounts<T>>::insert(sender.clone(), account);
            Self::_credit(sender, 10.into())
        }

        #[weight = 10_000]
        pub fn like_photo(_origin, photo: Vec<u8>) -> DispatchResult {
            let sender = ensure_signed(_origin)?;
            ensure!(<Photos<T>>::contains_key(photo.clone()), "Photo doesn't exist");
            let mut photo_info = Self::photos(photo.clone());
            for l in photo_info.clone().likes {
                ensure!(l != sender, "Already liked");
            }
            photo_info.likes.push(sender.clone());
            <Photos<T>>::insert(photo.clone(), photo_info);
            Self::_credit(sender, 1.into())
        }

        #[weight = 10_000]
        pub fn comment_photo(_origin, photo: Vec<u8>, comment: Vec<u8>) -> DispatchResult {
            let sender = ensure_signed(_origin)?;
            let mut photo_info = Self::photos(photo.clone());
            photo_info.comments.push(comment);
            <Photos<T>>::insert(photo.clone(), photo_info);
            Self::_credit(sender, 1.into())
        }

        #[weight = 10_000]
        pub fn edit_photo(_origin, photo: Vec<u8>, updated_photo: Vec<u8>) -> DispatchResult {
            let sender = ensure_signed(_origin)?;
            ensure!(<Photos<T>>::contains_key(photo.clone()), "Photo doesn't exist");
            let mut photo_info = Self::photos(photo.clone());
            for v in photo_info.clone().variants {
                ensure!(v != updated_photo, "Already has this variant");
            }
            photo_info.variants.push(updated_photo);
            <Photos<T>>::insert(photo.clone(), photo_info);
            Self::_credit(sender, 2.into())
        }

        #[weight = 10_000]
        pub fn create_affiliate(_origin, url: Vec<u8>, total_credit: T::TokenBalance, single_credit: T::TokenBalance) -> DispatchResult {
            let sender = ensure_signed(_origin)?;
            ensure!(!<Affiliations<T>>::contains_key((sender.clone(), url.clone())), "Affiliation already exist");
            Self::_transfer(sender.clone(), Self::treasury(), total_credit)?;
            <Affiliations<T>>::insert((sender, url), AffiliateInfo {total_credit, single_credit});
            Ok(())
        }

        #[weight = 10_000]
        pub fn claim_credit(_origin, founder: T::AccountId, url: Vec<u8>) -> DispatchResult {
            let sender = ensure_signed(_origin)?;
            ensure!(<Affiliations<T>>::contains_key((founder.clone(), url.clone())), "Affiliation doesn't exist");
            let mut affiliation_info = <Affiliations<T>>::get((founder.clone(), url.clone()));
            let updated_credit = affiliation_info.total_credit.checked_sub(&affiliation_info.single_credit).ok_or("overflow in affiliation credit")?;
            affiliation_info.total_credit = updated_credit;
            <Affiliations<T>>::insert((founder, url), affiliation_info.clone());
            Self::_transfer(Self::treasury(), sender.clone(), affiliation_info.single_credit)?;
            Ok(())
        }
    }
}

// implementation of mudule
// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
    // the ERC20 standard transfer function
    // internal
    fn _transfer(from: T::AccountId, to: T::AccountId, value: T::TokenBalance) -> DispatchResult {
        ensure!(<BalanceOf<T>>::contains_key(from.clone()), "Account does not own this token");
        let sender_balance = Self::balance_of(from.clone());
        ensure!(sender_balance >= value, "Not enough balance.");

        let updated_from_balance =
            sender_balance.checked_sub(&value).ok_or("overflow in calculating balance")?;
        let receiver_balance = Self::balance_of(to.clone());
        let updated_to_balance =
            receiver_balance.checked_add(&value).ok_or("overflow in calculating balance")?;

        // reduce sender's balance
        <BalanceOf<T>>::insert(from.clone(), updated_from_balance);

        // increase receiver's balance
        <BalanceOf<T>>::insert(to.clone(), updated_to_balance);

        Self::deposit_event(RawEvent::Transfer(from, to, value));
        Ok(())
    }

    fn _credit(to: T::AccountId, value: T::TokenBalance) -> DispatchResult {
        let receiver_balance = Self::balance_of(to.clone());
        let updated_to_balance =
            receiver_balance.checked_add(&value).ok_or("overflow in calculating balance")?;
        <BalanceOf<T>>::insert(to.clone(), updated_to_balance);
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
