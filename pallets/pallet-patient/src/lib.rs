#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{inherent::Vec, pallet_prelude::*, traits::Randomness};
	use frame_system::pallet_prelude::*;

	pub use pallet_access;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Patients<T: Config> {
		pub name: BoundedVec<u8, T::MaxNameLength>,
		pub email: BoundedVec<u8, T::MaxEmailLength>,
		pub data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
	}

	impl<T: Config> Default for Patients<T> {
		fn default() -> Self {
			Patients {
				name: BoundedVec::<u8, T::MaxNameLength>::default(),
				email: BoundedVec::<u8, T::MaxEmailLength>::default(),
				data_hash: None,
			}
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_access::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The type of Randomness we want to specify for this pallet.
		type RequestRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		#[pallet::constant]
		type MaxNameLength: Get<u32>;
		#[pallet::constant]
		type MaxEmailLength: Get<u32>;
		#[pallet::constant]
		type MaxHashLength: Get<u32>;
		#[pallet::constant]
		type MaxRequestList: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn patient_data)]
	pub type DataMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Patients<T>, OptionQuery>;

	#[pallet::storage]
	pub type RequestMap<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		T::AccountId,
		[u8; 16],
	>;

	#[pallet::storage]
	pub type AprovedRequestMap<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		T::AccountId,
		[u8; 16],
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// When Patient Data is Successfully registered.
		PatientDataUpdated {
			patient_account_id: T::AccountId,
			name: BoundedVec<u8, T::MaxNameLength>,
		},
		/// When a Request is Sucessfully added to the RequestQueue
		RequestQueued {
			requester: T::AccountId,
			patient_account_id: T::AccountId,
			request_unique_id: [u8; 16],
		},
		/// When a Request is Successfully Approved
		RequestApproved {
			requester: T::AccountId,
			patient_account_id: T::AccountId,
			request_unique_id: [u8; 16],
		},
		/// When a request is successfully executed
		RequestFulfilled {
			requester: T::AccountId,
			patient_account_id: T::AccountId,
			data_hash: BoundedVec<u8, T::MaxHashLength>,
		},
		/// When a request is successfully executed
		DataUpdated { requester: T::AccountId, patient_account_id: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		BoundsOverflow,
		AlreadyRegistered,
		OnlyOneRequestAllowed,
		AlreadyRequested,
		NoRequest,
		UnableToUpdate,
	}

	impl<T: Config> Pallet<T> {
		fn gen_request_id() -> [u8; 16] {
			// Create randomness
			let random = T::RequestRandomness::random(&b"unique_id"[..]).0;

			// Create randomness payload. Multiple Request can be generated in the same block,
			// retaining uniqueness.
			let unique_payload = (
				random,
				frame_system::Pallet::<T>::extrinsic_index().unwrap_or_default(),
				frame_system::Pallet::<T>::block_number(),
			);

			// Turns into a byte arrays
			let encoded_payload = unique_payload.encode();
			frame_support::Hashable::blake2_128(&encoded_payload)
		}

		fn register(
			patient_account: T::AccountId,
			name: Vec<u8>,
			email: Vec<u8>,
			data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> Result<(), DispatchError> {
			ensure!(!DataMap::<T>::contains_key(&patient_account), Error::<T>::AlreadyRegistered);

			let bounded_name: BoundedVec<u8, T::MaxNameLength> =
				name.try_into().map_err(|_| Error::<T>::BoundsOverflow)?;

			let bounded_email: BoundedVec<u8, T::MaxEmailLength> =
				email.try_into().map_err(|_| Error::<T>::BoundsOverflow)?;

			let patient =
				Patients::<T> { name: bounded_name.clone(), email: bounded_email, data_hash };

			DataMap::<T>::insert(&patient_account, patient);

			Self::deposit_event(Event::PatientDataUpdated {
				patient_account_id: patient_account,
				name: bounded_name,
			});

			Ok(())
		}

		fn request(
			request_unique_id: [u8; 16],
			requester: T::AccountId,
			patient_account_id: T::AccountId,
		) -> Result<(), DispatchError> {
			ensure!(
				!RequestMap::<T>::contains_key(&patient_account_id, &requester),
				Error::<T>::OnlyOneRequestAllowed
			);

			RequestMap::<T>::insert(&patient_account_id, &requester, &request_unique_id);

			Self::deposit_event(Event::RequestQueued {
				requester,
				patient_account_id,
				request_unique_id,
			});

			Ok(())
		}

		fn approve(
			patient_account_id: T::AccountId,
			requester: T::AccountId,
		) -> Result<(), DispatchError> {
			ensure!(
				RequestMap::<T>::contains_key(&patient_account_id, &requester),
				Error::<T>::NoRequest
			);

			let request_unique_id =
				(RequestMap::<T>::get(&patient_account_id, &requester)).unwrap();

			RequestMap::<T>::remove(&patient_account_id, &requester);

			AprovedRequestMap::<T>::insert(&patient_account_id, &requester, &request_unique_id);

			Self::deposit_event(Event::RequestApproved {
				requester,
				patient_account_id,
				request_unique_id,
			});

			Ok(())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		#[pallet::call_index(1)]
		pub fn register_patient_self(
			origin: OriginFor<T>,
			name: Vec<u8>,
			email: Vec<u8>,
			data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::register(sender.clone(), name, email, data_hash)?;

			Ok(())
		}

		#[pallet::weight(0)]
		#[pallet::call_index(2)]
		pub fn register_patient(
			origin: OriginFor<T>,
			patient_account_id: T::AccountId,
			name: Vec<u8>,
			email: Vec<u8>,
			data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;

			pallet_access::Pallet::<T>::validate(origin, sender.clone(), [0u8;32])?;

			Self::register(patient_account_id, name, email, data_hash)?;

			Ok(())
		}

		#[pallet::weight(0)]
		#[pallet::call_index(3)]
		pub fn request_patient_data(
			origin: OriginFor<T>,
			patient_account_id: T::AccountId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let request_unique_id = Self::gen_request_id();

			Self::request(request_unique_id, sender, patient_account_id)?;

			Ok(())
		}

		#[pallet::weight(0)]
		#[pallet::call_index(4)]
		pub fn approve_request(origin: OriginFor<T>, requester: T::AccountId) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::approve(sender, requester)?;

			Ok(())
		}

		#[pallet::weight(0)]
		#[pallet::call_index(5)]
		pub fn get_patient_data(
			origin: OriginFor<T>,
			patient_account_id: T::AccountId,
		) -> DispatchResult {
			let requester = ensure_signed(origin)?;

			ensure!(
				AprovedRequestMap::<T>::contains_key(&patient_account_id, &requester),
				Error::<T>::NoRequest
			);

			let data = DataMap::<T>::get(&patient_account_id).unwrap().data_hash.unwrap();
			// let s: String = data.iter().map(|x| x.encode()).collect::<Vec<_>>().concat().into();

			Self::deposit_event(Event::RequestFulfilled {
				requester,
				patient_account_id,
				data_hash: data,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		#[pallet::call_index(6)]
		pub fn update_patient_data(
			origin: OriginFor<T>,
			patient_account_id: T::AccountId,
			data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> DispatchResult {
			let requester = ensure_signed(origin.clone())?;

			ensure!(
				AprovedRequestMap::<T>::contains_key(&patient_account_id, &requester),
				Error::<T>::NoRequest
			);
			pallet_access::Pallet::<T>::validate(origin, requester.clone(), [0u8;32])?;

			let mut patient_data = DataMap::<T>::get(&patient_account_id).unwrap_or_default();

			patient_data.data_hash = data_hash;

			DataMap::<T>::insert(&patient_account_id, patient_data);

			Self::deposit_event(Event::DataUpdated { requester, patient_account_id });

			Ok(())
		}
	}
}
