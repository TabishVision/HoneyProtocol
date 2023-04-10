#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	pub use pallet_access;

	/// Struct Data Structure To Store Doctors personal data hash
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Doctors<T: Config> {
		pub personal_data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
	}

	impl<T: Config> Default for Doctors<T> {
		fn default() -> Self {
			Doctors { personal_data_hash: None }
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_access::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		///Hash length Bound For Bounded Vector
		#[pallet::constant]
		type MaxHashLength: Get<u32>;

		///Length Bound for Request and Approved Request List Length
		#[pallet::constant]
		type MaxListLength: Get<u32>;
	}

	///Storage Map for Storing Doctors Data against Account Id
	#[pallet::storage]
	#[pallet::getter(fn patient_data)]
	pub type DataMap<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Doctors<T>, OptionQuery>;

	///Storage Map for Storing all request made by Doctor Against their Account Id
	#[pallet::storage]
	#[pallet::getter(fn request_list)]
	pub type RequestMap<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::AccountId, T::MaxListLength>,
		ValueQuery,
	>;

	///Storage Map for Storing all approved requests for Doctors Against their Account Id
	#[pallet::storage]
	#[pallet::getter(fn approved_request_list)]
	pub type AprovedRequestMap<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::AccountId, T::MaxListLength>,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// When Doctor is Successfully registered or updated.
		DoctorDataUpdated { doctor_account_id: T::AccountId },
		/// When a Request is Sucessfully added to the RequestQueue
		RequestQueued { doctor_account_id: T::AccountId, patient_account_id: T::AccountId },
		/// When a Request is Successfull Approved
		RequestApproved { doctor_account_id: T::AccountId, patient_account_id: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		AlreadyRegistered,
		AlreadyRequested,
		NoRequest,
		UnableToUpdate,
		AlreadyApproved,
		MaxListLengthReached,
	}

	impl<T: Config> Pallet<T> {
		fn register_self(
			doctor_account_id: T::AccountId,
			personal_data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> Result<(), DispatchError> {
			ensure!(!DataMap::<T>::contains_key(&doctor_account_id), Error::<T>::AlreadyRegistered);

			let doctor = Doctors::<T> { personal_data_hash };

			DataMap::<T>::insert(&doctor_account_id, doctor);

			Self::deposit_event(Event::DoctorDataUpdated { doctor_account_id });

			Ok(())
		}

		pub fn add_request(
			requester: T::AccountId,
			patient_account_id: T::AccountId,
		) -> DispatchResult {
			// Get the current BoundedVec associated with the requester
			let patient_ids = RequestMap::<T>::get(&requester);

			// Ensure the patient_account_id is not already in the list
			ensure!(
				!patient_ids.iter().any(|account_id| account_id == &patient_account_id),
				Error::<T>::AlreadyRequested
			);

			// Append collectible to OwnerOfCollectibles map
			RequestMap::<T>::try_append(&requester, patient_account_id.clone())
				.map_err(|_| Error::<T>::MaxListLengthReached)?;

			Self::deposit_event(Event::RequestQueued {
				doctor_account_id: requester,
				patient_account_id,
			});

			Ok(())
		}

		fn remove_request(
			requester: T::AccountId,
			patient_account_id: T::AccountId,
		) -> Result<(), DispatchError> {
			let mut patient_ids = RequestMap::<T>::get(&requester);

			ensure!(
				patient_ids.iter().any(|account_id| account_id == &patient_account_id),
				Error::<T>::NoRequest
			);

			if let Some(ind) = patient_ids.iter().position(|id| id == &patient_account_id) {
				patient_ids.swap_remove(ind);
			} else {
				return Err(Error::<T>::NoRequest.into())
			}

			RequestMap::<T>::insert(&requester, patient_ids);

			Ok(())
		}

		pub fn add_approved_request(
			patient_account_id: T::AccountId,
			requester: T::AccountId,
		) -> Result<(), DispatchError> {
			Self::remove_request(requester.clone(), patient_account_id.clone())?;

			let approved_patient_ids = AprovedRequestMap::<T>::get(&requester);

			ensure!(
				!approved_patient_ids.iter().any(|account_id| account_id == &patient_account_id),
				Error::<T>::AlreadyApproved
			);

			AprovedRequestMap::<T>::try_append(&requester, patient_account_id.clone())
				.map_err(|_| Error::<T>::MaxListLengthReached)?;

			Self::deposit_event(Event::RequestApproved {
				doctor_account_id: requester,
				patient_account_id,
			});

			Ok(())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		#[pallet::call_index(1)]
		pub fn register(
			origin: OriginFor<T>,
			personal_data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::register_self(sender, personal_data_hash)?;

			Ok(())
		}
	}
}
