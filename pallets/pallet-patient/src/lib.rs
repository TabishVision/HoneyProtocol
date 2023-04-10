#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	pub use pallet_access;
	pub use pallet_doctor;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Patients<T: Config> {
		pub personal_data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		pub data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
	}

	impl<T: Config> Default for Patients<T> {
		fn default() -> Self {
			Patients { personal_data_hash: None, data_hash: None }
		}
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_access::Config + pallet_doctor::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::storage]
	#[pallet::getter(fn patient_data)]
	pub type DataMap<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Patients<T>, OptionQuery>;

	///Storage Map for Storing all doctors who made request to view or update data against patient
	/// AccountId
	#[pallet::storage]
	#[pallet::getter(fn request_list)]
	pub type RequestMap<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<T::AccountId, T::MaxListLength>,
		ValueQuery,
	>;

	///Storage Map for Storing all approved requests for Patients Against their Account Id
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
		/// When Patient Data is Successfully registered.
		PatientDataUpdated { patient_account_id: T::AccountId },
		/// When a Request is Sucessfully added to the RequestQueue
		RequestQueued { requester: T::AccountId, patient_account_id: T::AccountId },
		/// When a Request is Successfully Approved
		RequestApproved { requester: T::AccountId, patient_account_id: T::AccountId },
		/// When a request is successfully executed
		DataUpdated { requester: T::AccountId, patient_account_id: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		BoundsOverflow,
		AlreadyRegistered,
		AlreadyRequested,
		AlreadyApproved,
		NoRequest,
		UnableToUpdate,
		MaxListLengthReached,
		NotApproved,
		NoPatient,
	}

	impl<T: Config> Pallet<T> {
		fn register(
			patient_account_id: T::AccountId,
			personal_data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
			data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> Result<(), DispatchError> {
			ensure!(
				!DataMap::<T>::contains_key(&patient_account_id),
				Error::<T>::AlreadyRegistered
			);

			let patient = Patients::<T> { personal_data_hash, data_hash };

			DataMap::<T>::insert(&patient_account_id, patient);

			Self::deposit_event(Event::PatientDataUpdated { patient_account_id });

			Ok(())
		}

		fn request(
			requester: T::AccountId,
			patient_account_id: T::AccountId,
		) -> Result<(), DispatchError> {
			ensure!(DataMap::<T>::contains_key(&patient_account_id), Error::<T>::NoPatient);

			let doctors_ids = RequestMap::<T>::get(&patient_account_id);

			ensure!(
				!doctors_ids.iter().any(|account_id| account_id == &requester),
				Error::<T>::AlreadyRequested
			);

			let approved_doctor_ids = AprovedRequestMap::<T>::get(&patient_account_id);

			ensure!(
				!approved_doctor_ids.iter().any(|account_id| account_id == &requester),
				Error::<T>::AlreadyApproved
			);

			RequestMap::<T>::try_append(&patient_account_id, requester.clone())
				.map_err(|_| Error::<T>::MaxListLengthReached)?;

			pallet_doctor::Pallet::<T>::add_request(requester.clone(), patient_account_id.clone())?;

			Self::deposit_event(Event::RequestQueued { requester, patient_account_id });

			Ok(())
		}

		fn remove_request(
			patient_account_id: T::AccountId,
			requester: T::AccountId,
		) -> Result<(), DispatchError> {
			let mut doctor_ids = RequestMap::<T>::get(&patient_account_id);

			ensure!(
				doctor_ids.iter().any(|account_id| account_id == &requester),
				Error::<T>::NoRequest
			);

			if let Some(ind) = doctor_ids.iter().position(|id| id == &requester) {
				doctor_ids.swap_remove(ind);
			} else {
				return Err(Error::<T>::NoRequest.into())
			}

			RequestMap::<T>::insert(&requester, doctor_ids);

			Ok(())
		}

		fn approve(
			patient_account_id: T::AccountId,
			requester: T::AccountId,
		) -> Result<(), DispatchError> {
			Self::remove_request(patient_account_id.clone(), requester.clone())?;

			let approved_doctor_ids = AprovedRequestMap::<T>::get(&patient_account_id);

			ensure!(
				!approved_doctor_ids.iter().any(|account_id| account_id == &requester),
				Error::<T>::AlreadyApproved
			);

			AprovedRequestMap::<T>::try_append(&patient_account_id, requester.clone())
				.map_err(|_| Error::<T>::MaxListLengthReached)?;

			pallet_doctor::Pallet::<T>::add_approved_request(
				patient_account_id.clone(),
				requester.clone(),
			)?;

			Self::deposit_event(Event::RequestApproved { requester, patient_account_id });

			Ok(())
		}

		fn update(
			patient_account_id: T::AccountId,
			requester: T::AccountId,
			data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
			personal_data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> Result<(), DispatchError> {
			let approved_doctor_ids = AprovedRequestMap::<T>::get(&patient_account_id);

			ensure!(
				approved_doctor_ids.iter().any(|account_id| account_id == &requester),
				Error::<T>::NotApproved
			);

			let mut patient_data = DataMap::<T>::get(&patient_account_id).unwrap_or_default();

			patient_data.data_hash = data_hash;
			patient_data.personal_data_hash = personal_data_hash;

			DataMap::<T>::insert(&patient_account_id, patient_data);

			Self::deposit_event(Event::DataUpdated { requester, patient_account_id });

			Ok(())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		#[pallet::call_index(1)]
		pub fn register_patient_self(
			origin: OriginFor<T>,
			personal_data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
			data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::register(sender.clone(), personal_data_hash, data_hash)?;

			Ok(())
		}

		#[pallet::weight(0)]
		#[pallet::call_index(2)]
		pub fn register_patient(
			origin: OriginFor<T>,
			patient_account_id: T::AccountId,
			personal_data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
			data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;

			pallet_access::Pallet::<T>::has_role(origin, sender.clone(), [0u8; 32])?;

			Self::register(patient_account_id, personal_data_hash, data_hash)?;

			Ok(())
		}

		#[pallet::weight(0)]
		#[pallet::call_index(3)]
		pub fn request_patient_data(
			origin: OriginFor<T>,
			patient_account_id: T::AccountId,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;

			pallet_access::Pallet::<T>::has_role(origin, sender.clone(), [0u8; 32])?;

			Self::request(sender, patient_account_id)?;

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
		pub fn update_patient_data(
			origin: OriginFor<T>,
			patient_account_id: T::AccountId,
			data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
			personal_data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> DispatchResult {
			let requester = ensure_signed(origin.clone())?;

			pallet_access::Pallet::<T>::has_role(origin, requester.clone(), [0u8; 32])?;

			Self::update(patient_account_id, requester, data_hash, personal_data_hash)?;

			Ok(())
		}
	}
}
