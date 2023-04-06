#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
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
	pub type RequestMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<T::AccountId, T::MaxListLength>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn approved_request_list)]
	pub type AprovedRequestMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<T::AccountId, T::MaxListLength>, ValueQuery>;

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
		AlreadyApproved,
		MaxListLengthReached,
		BoundedVecError,
	}

	impl <T:Config> Pallet<T> {
		fn register_self(
			doctor_account_id: T::AccountId,
			personal_data_hash: Option<BoundedVec<u8, T::MaxHashLength>>,
		) -> Result<(), DispatchError> {

			ensure!(!DataMap::<T>::contains_key(&doctor_account_id), Error::<T>::AlreadyRegistered);

			let doctor =
				Doctors::<T> { personal_data_hash };

			DataMap::<T>::insert(&doctor_account_id, doctor);

			Self::deposit_event(Event::DoctorDataUpdated { doctor_account_id } );

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

			Ok(())
		}

		fn remove_request(
			requester: T::AccountId,
			patient_account_id: T::AccountId
		)	-> Result<(), DispatchError> {

			let mut patient_ids = RequestMap::<T>::get(&requester);

			if let Some(ind) = patient_ids.iter().position(|id| id == &patient_account_id) {
            patient_ids.swap_remove(ind);
            } else {
            return Err(Error::<T>::NoRequest.into())
            }

			// patient_ids.try_push(patient_account_id.clone()).map_err(|_| Error::<T>::MaxListLengthReached)?;


			let approved_patient_ids = AprovedRequestMap::<T>::get(&requester);

			ensure!(patient_ids.iter().any(|account_id|  account_id == &patient_account_id), Error::<T>::NoRequest);

			ensure!(!approved_patient_ids.iter().any(|account_id|  account_id == &patient_account_id), Error::<T>::AlreadyApproved);

			Ok(())
		}

		// fn add_approved_request(
		// 	patient_account_id: T::AccountId,
		// 	requester: T::AccountId,
		// ) -> Result<(), DispatchError> {

		// 	let patient_ids = RequestMap::<T>::get(&requester);
		// 	ensure!(
		// 		patient_ids.iter().any(|account_id|  account_id == &patient_account_id),
		// 		Error::<T>::NoRequest
		// 	);

		// 	let approved_patient_ids = AprovedRequestMap::<T>::get(&requester).unwrap();


		// 	// new_approved_list.push(patient_account_id.clone());

		// 	AprovedRequestMap::<T>::insert(&requester, approved_patient_ids);

		// 	Self::deposit_event(Event::ApprovedRequestAdded {
		// 		patient_account_id,
		// 		requester
		// 	});

		// 	Ok(())
		// }
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
