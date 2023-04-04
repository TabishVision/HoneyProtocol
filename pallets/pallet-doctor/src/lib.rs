#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{inherent::Vec, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	pub use pallet_access;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Doctors<T: Config> {
		pub personal_data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
	}

	impl<T: Config> Default for Doctors<T> {
		fn default() -> Self {
			Doctors {
				personal_data_hash: None,
			}
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_access::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		#[pallet::constant]
		type MaxHashLength: Get<u32>;
        
        #[pallet::constant]
		type MaxListLength: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn patient_data)]
	pub type DataMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Doctors<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn request_list)]
	pub type RequestMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Vec<T::AccountId>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn approved_request_list)]
	pub type AprovedRequestMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Vec<T::AccountId>, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// When Doctor is Successfully registered/updated.
		DoctorDataUpdated {
			doctor_account_id: T::AccountId,
		},
		/// When a Request is Sucessfully added to the RequestQueue
		RequestQueued {
			patient_account_id: T::AccountId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		BoundsOverflow,
		AlreadyRegistered,
		AlreadyRequested,
		NoRequest,
		UnableToUpdate,
	}

}
