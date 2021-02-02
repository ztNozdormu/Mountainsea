#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use frame_support::{
	decl_module, decl_storage, decl_error, decl_event, ensure, StorageValue, StorageMap, Parameter,
	dispatch::DispatchResult,traits::{Randomness, Currency, ExistenceRequirement, ReservableCurrency},
};
use sp_io::hashing::blake2_128;
use frame_system::{self as system, ensure_signed};
use sp_runtime::{DispatchError,RuntimeDebug, traits::{AtLeast32Bit, Bounded, Member},};
use crate::linked_item::{LinkedList, LinkedItem};
use sp_std::vec;

mod linked_item;

// 定义HelloKitty数据结构--元组结构体，其成员为数组长度为16，数组元素属性为u8
#[derive(Encode , Decode, RuntimeDebug, Clone, PartialEq)]
pub struct HelloKitty(pub [u8;16]);

// 继承substrate框架Trait接口  并注入相关类型
pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type KittyIndex: Parameter  + Member + AtLeast32Bit + Bounded + Default + Copy;
	type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
	type Randomness: Randomness<Self::Hash>;
}
// 注入自定义类型 以及框架第三方类型 资产管理模块：BalanceOf
type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
type KittyLinkedItem<T> = LinkedItem<<T as Trait>::KittyIndex>;
type OwnedKittiesList<T> = LinkedList<OwnedKitties<T>, <T as system::Trait>::AccountId, <T as Trait>::KittyIndex>;


// 自定义存储结构 （根据具体业务需求进行设计）
decl_storage! {
	trait Store for Module<T: Trait> as Kitties {
		/// Stores all the kitties, key is the kitty id / index
		pub Kitties get(fn kitties): map hasher(blake2_128_concat) T::KittyIndex => Option<HelloKitty>;
		/// Stores the total number of kitties. i.e. the next kitty index
		pub KittiesCount get(fn kitties_count): T::KittyIndex;

		/// Store owned kitties in a linked list.
		pub OwnedKitties get(fn owned_kitties): map hasher(blake2_128_concat) (T::AccountId, Option<T::KittyIndex>) => Option<KittyLinkedItem<T>>;
		/// Store owner of each kitity.
		pub KittyOwners get(fn kitty_owner): map hasher(blake2_128_concat) T::KittyIndex => Option<T::AccountId>;

		/// Get kitty price. None means not for sale.
		pub KittyPrices get(fn kitty_price): map hasher(blake2_128_concat) T::KittyIndex => Option<BalanceOf<T>>;

		//map: AccountId-> [KittyIndex1, KittyIndex2 ...]
		pub KittyTotal get(fn kitty_total) : map hasher(blake2_128_concat) T::AccountId => vec::Vec<T::KittyIndex>;

		//map: KittyIndex-> (Parent1,Parent2)
        pub KittiesParents get(fn kitty_parents) : map hasher(blake2_128_concat) T::KittyIndex => (T::KittyIndex, T::KittyIndex);

        //map: KittyIndex-> [Children1, Children2 ...]
        pub KittiesChildren get(fn kitty_children): double_map hasher(blake2_128_concat) T::KittyIndex,  hasher(blake2_128_concat) T::KittyIndex => vec::Vec<T::KittyIndex>;

		//map: KittyIndex-> [Sibling1, Sibling2 ...]
        pub KittiesSibling get(fn kitty_sibling): map hasher(blake2_128_concat) T::KittyIndex => vec::Vec<T::KittyIndex>;

        //map: KittyIndex-> [Partner1, Partner2 ...]
        pub KittiesPartner get(fn kitty_partner) : map hasher(blake2_128_concat) T::KittyIndex => vec::Vec<T::KittyIndex>;
	}
}
// 自定义错误类型
decl_error! {
	pub enum Error for Module<T: Trait> {
		KittiesCountOverflow,
		InvalidKittyId,
		RequireDifferentParent,
		RequireOwner,
		NotForSale,
		PriceTooLow,
		TransferToSelf, 
	}
}

// 自定义事件类型
decl_event!(
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		<T as Trait>::KittyIndex,
		Balance = BalanceOf<T>,
		BlockNumber = <T as frame_system::Trait>::BlockNumber,
	{
		/// A kitty is created. (owner, kitty_id)
		Created(AccountId, KittyIndex),
		/// A kitty is transferred. (from, to, kitty_id)
		Transferred(AccountId, AccountId, KittyIndex),
		/// A kitty is available for sale. (owner, kitty_id, price)
		Ask(AccountId, KittyIndex, Option<Balance>),
		/// A kitty is sold. (from, to, kitty_id, price)
		Sold(AccountId, AccountId, KittyIndex, Balance),

		LockFunds(AccountId, Balance, BlockNumber),
		UnlockFunds(AccountId, Balance, BlockNumber),
		// sender, dest, amount, block number
		TransferFunds(AccountId, AccountId, Balance, BlockNumber),
	}
);

// 为pallet实现的调用函数
// 参考文档：https://shimo.im/docs/Q6HwhRkHvHt8TCRY/read
decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Create a new kitty
		#[weight = 0]
		pub fn create(origin, amount: BalanceOf<T>) {
			let originC = origin.clone();
			let sender = ensure_signed(origin)?;
			let kitty_id = Self::next_kitty_id()?;

			// Generate a random 128bit value
			let dna = Self::random_value(&sender);

			// Create and store kitty
			let hello_kitty = HelloKitty(dna);
			Self::insert_kitty(&sender, kitty_id, hello_kitty);

			Self::reserve_funds(originC, amount);

			Self::deposit_event(RawEvent::Created(sender, kitty_id));
		}

		/// Breed kitties
		#[weight = 0]
		pub fn breed(origin, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex, amount: BalanceOf<T>) {
			let originC = origin.clone();
			let sender = ensure_signed(origin)?;

			let new_kitty_id = Self::do_breed(&sender, kitty_id_1, kitty_id_2)?;

			Self::reserve_funds(originC, amount);

			Self::deposit_event(RawEvent::Created(sender, new_kitty_id));
		}

		/// Transfer a kitty to new owner
		#[weight = 0]
		pub fn transfer(origin, to: T::AccountId, kitty_id: T::KittyIndex, amount: BalanceOf<T>) {
			let originC = origin.clone();
			let sender = ensure_signed(origin)?;

			ensure!(<OwnedKitties<T>>::contains_key((&sender, Some(kitty_id))), Error::<T>::RequireOwner);

			Self::do_transfer(&sender, &to, kitty_id);

			Self::unreserve_funds(originC, amount);

			Self::deposit_event(RawEvent::Transferred(sender, to, kitty_id));
		}

		/// Set a price for a kitty for sale
		/// None to delist the kitty
		#[weight = 0]
 		pub fn ask(origin, kitty_id: T::KittyIndex, new_price: Option<BalanceOf<T>>) {
			let sender = ensure_signed(origin)?;

			ensure!(<OwnedKitties<T>>::contains_key((&sender, Some(kitty_id))), Error::<T>::RequireOwner);

			<KittyPrices<T>>::mutate_exists(kitty_id, |price| *price = new_price);

			Self::deposit_event(RawEvent::Ask(sender, kitty_id, new_price));
		}

		/// Buy a kitty
		#[weight = 0]
		pub fn buy(origin, kitty_id: T::KittyIndex, price: BalanceOf<T>) {
			let sender = ensure_signed(origin)?;

			let owner = Self::kitty_owner(kitty_id).ok_or(Error::<T>::InvalidKittyId)?;

			let kitty_price = Self::kitty_price(kitty_id).ok_or(Error::<T>::NotForSale)?;

			ensure!(price >= kitty_price, Error::<T>::PriceTooLow);

			T::Currency::transfer(&sender, &owner, kitty_price, ExistenceRequirement::KeepAlive)?;

			<KittyPrices<T>>::remove(kitty_id);

			Self::do_transfer(&owner, &sender, kitty_id);

			Self::deposit_event(RawEvent::Sold(owner, sender, kitty_id, kitty_price));
		}

		/// Reserves the specified amount of funds from the caller
		#[weight = 0]
		pub fn reserve_funds(origin, amount: BalanceOf<T>) -> DispatchResult {
			let locker = ensure_signed(origin)?;

			T::Currency::reserve(&locker, amount)
					.map_err(|_| "locker can't afford to lock the amount requested")?;

			let now = <system::Module<T>>::block_number();

			Self::deposit_event(RawEvent::LockFunds(locker, amount, now));
			Ok(())
		}

		/// Unreserves the specified amount of funds from the caller
		#[weight = 0]
		pub fn unreserve_funds(origin, amount: BalanceOf<T>) -> DispatchResult {
			let unlocker = ensure_signed(origin)?;

			T::Currency::unreserve(&unlocker, amount);
			// ReservableCurrency::unreserve does not fail (it will lock up as much as amount)

			let now = <system::Module<T>>::block_number();

			Self::deposit_event(RawEvent::UnlockFunds(unlocker, amount, now));
			Ok(())
		}

		/// Transfers funds. Essentially a wrapper around the Currency's own transfer method
		#[weight = 0]
		pub fn transfer_funds(origin, dest: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			T::Currency::transfer(&sender, &dest, amount, ExistenceRequirement::AllowDeath)?;

			let now = <system::Module<T>>::block_number();

			Self::deposit_event(RawEvent::TransferFunds(sender, dest, amount, now));
			Ok(())
		}

		/// Atomically unreserves funds and and transfers them.
		/// might be useful in closed economic systems
		#[weight = 0]
		pub fn unreserve_and_transfer(
			origin,
			to_punish: T::AccountId,
			dest: T::AccountId,
			collateral: BalanceOf<T>
		) -> DispatchResult {
			let _ = ensure_signed(origin)?; // dangerous because can be called with any signature (so dont do this in practice ever!)

						// If collateral is bigger than to_punish's reserved_balance, store what's left in overdraft.
			let overdraft = T::Currency::unreserve(&to_punish, collateral);

			T::Currency::transfer(&to_punish, &dest, collateral - overdraft, ExistenceRequirement::AllowDeath)?;

			let now = <system::Module<T>>::block_number();
			Self::deposit_event(RawEvent::TransferFunds(to_punish, dest, collateral - overdraft, now));

			Ok(())
		}
	}
}

// 当前pallet module实现的行为方法
impl<T: Trait> Module<T> {
	fn random_value(sender: &T::AccountId) -> [u8; 16] {
		let payload = (
			T::Randomness::random_seed(),
			&sender,
			<frame_system::Module<T>>::extrinsic_index(),
		);
		payload.using_encoded(blake2_128)
	}

	fn next_kitty_id() -> sp_std::result::Result<T::KittyIndex, DispatchError> {
		let kitty_id = Self::kitties_count();
		if kitty_id == T::KittyIndex::max_value() {
			return Err(Error::<T>::KittiesCountOverflow.into());
		}
		Ok(kitty_id)
	}

	fn insert_owned_kitty(owner: &T::AccountId, kitty_id: T::KittyIndex) {
		<OwnedKittiesList<T>>::append(owner, kitty_id);
		<KittyOwners<T>>::insert(kitty_id, owner);
	}

	fn insert_kitty(owner: &T::AccountId, kitty_id: T::KittyIndex, hello_kitty: HelloKitty) {
		// Create and store hello_kitty
		Kitties::<T>::insert(kitty_id, hello_kitty);
		// TODO 加1 1.into()
		let incre_1 = 1 as u32;
		KittiesCount::<T>::put(kitty_id + incre_1.into());
		Self::insert_owned_kitty(owner, kitty_id);
	}
	// 记录父母
	fn update_kitties_parents(
		children: T::KittyIndex,
		father: T::KittyIndex,
		mother: T::KittyIndex,
	) {
		<KittiesParents<T>>::insert(children, (father, mother));
	}

	// 记录孩子
	fn update_kitties_children(
		children: T::KittyIndex,
		father: T::KittyIndex,
		mother: T::KittyIndex,
	) {
		if <KittiesChildren<T>>::contains_key(father, mother) {
			let _ = <KittiesChildren<T>>::mutate(father, mother, |val| val.push(children));
		} else {
			<KittiesChildren<T>>::insert(father, mother, vec![children]);
		}
	}

	// 记录兄弟姐妹
	fn update_kitties_sibling(kitty_id: T::KittyIndex) {
		let (father, mother) = KittiesParents::<T>::get(kitty_id);
		
		// 如果父母之前生过兄弟姐妹
		if <KittiesChildren<T>>::contains_key(father, mother) {
			let val: vec::Vec<T::KittyIndex> = KittiesChildren::<T>::get(father, mother);
			let reserve_val: vec::Vec<T::KittyIndex> =
				val.into_iter().filter(|&val| val != kitty_id).collect();
			<KittiesSibling<T>>::insert(kitty_id, reserve_val);
		} else {
			// 如果没有兄弟姐妹
			<KittiesSibling<T>>::insert(kitty_id, vec::Vec::<T::KittyIndex>::new());
		}
	}
	// 更新配偶
	fn update_kitties_partner(partner1: T::KittyIndex, partner2: T::KittyIndex) {
		if KittiesPartner::<T>::contains_key(&partner1){
			let val: vec::Vec<T::KittyIndex> = KittiesPartner::<T>::get(partner1);
			let reserve_val: vec::Vec<T::KittyIndex> =
				val.into_iter().filter(|&val| val == partner2).collect();
			if reserve_val.len() == 0 {
				KittiesPartner::<T>::mutate(&partner1, |val| val.push(partner2));
			}
		}else{
			KittiesPartner::<T>::insert(partner1, vec![partner2]);
		};
	}
	fn do_breed(sender: &T::AccountId, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) -> sp_std::result::Result<T::KittyIndex, DispatchError> {
		let kitty1 = Self::kitties(kitty_id_1).ok_or(Error::<T>::InvalidKittyId)?;
		let kitty2 = Self::kitties(kitty_id_2).ok_or(Error::<T>::InvalidKittyId)?;

		ensure!(<OwnedKitties<T>>::contains_key((&sender, Some(kitty_id_1))), Error::<T>::RequireOwner);
		ensure!(<OwnedKitties<T>>::contains_key((&sender, Some(kitty_id_2))), Error::<T>::RequireOwner);
		ensure!(kitty_id_1 != kitty_id_2, Error::<T>::RequireDifferentParent);

		let kitty_id = Self::next_kitty_id()?;

		let kitty1_dna = kitty1.0;
		let kitty2_dna = kitty2.0;

		// Generate a random 128bit value
		let selector = Self::random_value(&sender);
		let mut new_dna = [0u8; 16];

		// Combine parents and selector to create new kitty
		for i in 0..kitty1_dna.len() {
			new_dna[i] = combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
		}
        // save new kitty
		Self::insert_kitty(sender, kitty_id, HelloKitty(new_dna));

		// 更新配偶
        Self::update_kitties_partner(kitty_id_1, kitty_id_2);
        Self::update_kitties_partner(kitty_id_2,kitty_id_1);

        // 记录父母
        Self::update_kitties_parents(kitty_id, kitty_id_1, kitty_id_2);

        // 记录孩子
        Self::update_kitties_children(kitty_id, kitty_id_1, kitty_id_2);

       // 记录兄弟姐妹
        Self::update_kitties_sibling(kitty_id);

		Ok(kitty_id)
	}

	fn do_transfer(from: &T::AccountId, to: &T::AccountId, kitty_id: T::KittyIndex)  {
		<OwnedKittiesList<T>>::remove(&from, kitty_id);
		Self::insert_owned_kitty(&to, kitty_id);
	}
}


// 公共工具方法
fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
	(selector & dna1) | (!selector & dna2)
}

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use sp_core::H256;
	use frame_support::{impl_outer_origin, parameter_types, weights::Weight, traits::{OnInitialize,OnFinalize}};
	use sp_runtime::{
		traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
	};
    use frame_system as system;

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq, Debug)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: Weight = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	}
	impl system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type DbWeight = ();
		type BlockExecutionWeight = ();
		type ExtrinsicBaseWeight = ();
		type MaximumExtrinsicWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
		type AccountData = pallet_balances::AccountData<u64>;
		type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type BaseCallFilter = ();
        type PalletInfo = ();
	}

	parameter_types! {
        pub const ExistentialDeposit: u64 = 0;	
        pub const MaxLocks: u32 = 0;
	}

	impl pallet_balances::Trait for Test {
		type Balance = u64;
		type Event = ();
        type DustRemoval = ();
        type MaxLocks = MaxLocks;
		type ExistentialDeposit = ExistentialDeposit;
        type AccountStore = system::Module<Test>;
        type WeightInfo = ();
	}

	impl Trait for Test {
		type KittyIndex = u32;
		type Currency = pallet_balances::Module<Test>;
		type Randomness = pallet_randomness_collective_flip::Module<Test>;
		type Event = ();
	}
	
	type OwnedKittiesTest = OwnedKitties<Test>;
	type Kitties = Module<Test>;
	type System = frame_system::Module<Test>;

	fn run_to_block(n: u64) {
		while System::block_number() < n {
			Kitties::on_finalize(System::block_number());
			System::on_finalize(System::block_number());
			System::set_block_number(System::block_number() + 1);
			System::on_initialize(System::block_number());
			Kitties::on_initialize(System::block_number());
		}
	}

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sp_io::TestExternalities {
		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}

	#[test]
	fn owned_kitties_can_append_values() {
		new_test_ext().execute_with(|| {
			OwnedKittiesList::<Test>::append(&0, 1);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: Some(1),
				next: Some(1),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: None,
			}));

			OwnedKittiesList::<Test>::append(&0, 2);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: Some(2),
				next: Some(1),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: Some(2),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), Some(KittyLinkedItem::<Test> {
				prev: Some(1),
				next: None,
			}));

			OwnedKittiesList::<Test>::append(&0, 3);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: Some(3),
				next: Some(1),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: Some(2),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), Some(KittyLinkedItem::<Test> {
				prev: Some(1),
				next: Some(3),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(3))), Some(KittyLinkedItem::<Test> {
				prev: Some(2),
				next: None,
			}));
		});
	}

	#[test]
	fn owned_kitties_can_remove_values() {
		new_test_ext().execute_with(|| {
			OwnedKittiesList::<Test>::append(&0, 1);
			OwnedKittiesList::<Test>::append(&0, 2);
			OwnedKittiesList::<Test>::append(&0, 3);

			OwnedKittiesList::<Test>::remove(&0, 2);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: Some(3),
				next: Some(1),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: Some(3),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), None);

			assert_eq!(OwnedKittiesTest::get(&(0, Some(3))), Some(KittyLinkedItem::<Test> {
				prev: Some(1),
				next: None,
			}));

			OwnedKittiesList::<Test>::remove(&0, 1);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: Some(3),
				next: Some(3),
			}));

			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), None);

			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), None);

			assert_eq!(OwnedKittiesTest::get(&(0, Some(3))), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: None,
			}));

			OwnedKittiesList::<Test>::remove(&0, 3);

			assert_eq!(OwnedKittiesTest::get(&(0, None)), Some(KittyLinkedItem::<Test> {
				prev: None,
				next: None,
			}));
			assert_eq!(OwnedKittiesTest::get(&(0, Some(1))), None);
			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), None);
			assert_eq!(OwnedKittiesTest::get(&(0, Some(2))), None);
		});
	}

	#[test]
	fn owned_kitties_create() {
		new_test_ext().execute_with(|| {
			run_to_block(10);
			assert_eq!(Kitties::create(Origin::signed(1), 100), Ok(()));
		});
	}

	#[test]
	fn owned_kitties_breed() {
		new_test_ext().execute_with(|| {
			run_to_block(10);
			assert_eq!(Kitties::create(Origin::signed(1), 100), Ok(()));
			assert_eq!(Kitties::create(Origin::signed(1), 100), Ok(()));
			assert_eq!(Kitties::breed(Origin::signed(1), 0, 1, 100), Ok(()));
		});
	}

	#[test]
	fn owned_kitties_transfer() {
		new_test_ext().execute_with(|| {
			run_to_block(10);
			assert_eq!(Kitties::create(Origin::signed(1), 100), Ok(()));
			assert_eq!(Kitties::transfer(Origin::signed(1), 2, 0, 100), Ok(()));
		});
	}

	#[test]
	fn owned_kitties_ask() {
		new_test_ext().execute_with(|| {
			run_to_block(10);
			assert_eq!(Kitties::create(Origin::signed(1), 100), Ok(()));
			assert_eq!(Kitties::ask(Origin::signed(1), 0, Some(100)), Ok(()));
		});
	}

	#[test]
	fn owned_kitties_buy() {
		new_test_ext().execute_with(|| {
			run_to_block(10);
			assert_eq!(Kitties::create(Origin::signed(1), 100), Ok(()));
			assert_eq!(Kitties::ask(Origin::signed(1), 0, Some(100)), Ok(()));
			assert_eq!(Kitties::buy(Origin::signed(1), 0, 110), Ok(()));
		});
	}
}