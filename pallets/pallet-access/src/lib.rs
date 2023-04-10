#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::storage]
	type Roles<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 32], ()>;

	#[pallet::storage]
	type MemberRoles<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, [u8; 32], Blake2_128Concat, T::AccountId, bool>;

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub roles: Vec<[u8; 32]>,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self { roles: Vec::new() }
		}
	}

	// The build of genesis for the pallet.
	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			for role in &self.roles {
				Roles::<T>::insert(role, ());
			}
		}
	}

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RoleAssigned { user: T::AccountId, role: [u8; 32] },
		RoleRevoked { user: T::AccountId, role: [u8; 32] },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		AccessDenied,
		AlreadyHasRole,
		InvalidRole,
		NotAssigned,
	}

	impl<T: Config> Pallet<T> {
		// Generates and returns the unique_id and color
		fn assign_role(user: T::AccountId, new_role: [u8; 32]) -> Result<(), DispatchError> {
			ensure!(Roles::<T>::contains_key(&new_role), Error::<T>::InvalidRole);

			if MemberRoles::<T>::contains_key(&new_role, &user) {
				ensure!(
					!MemberRoles::<T>::get(&new_role, &user).unwrap(),
					Error::<T>::AlreadyHasRole
				);
			}

			MemberRoles::<T>::insert(new_role, user.clone(), true);

			Self::deposit_event(Event::RoleAssigned { user: user.clone(), role: new_role });

			Ok(())
		}

		fn revoke_role(user: T::AccountId, new_role: [u8; 32]) -> Result<(), DispatchError> {
			ensure!(Roles::<T>::contains_key(&new_role), Error::<T>::InvalidRole);

			ensure!(MemberRoles::<T>::contains_key(&new_role, &user), Error::<T>::NotAssigned);

			ensure!(MemberRoles::<T>::get(&new_role, &user).unwrap(), Error::<T>::NotAssigned);

			MemberRoles::<T>::insert(&new_role, &user, false);

			Self::deposit_event(Event::RoleRevoked { user: user.clone(), role: new_role });

			Ok(())
		}

		fn validate_role(user: T::AccountId, new_role: [u8; 32]) -> Result<(), DispatchError> {
			ensure!(Roles::<T>::contains_key(&new_role), Error::<T>::InvalidRole);

			ensure!(MemberRoles::<T>::contains_key(&new_role, &user), Error::<T>::NotAssigned);

			ensure!(MemberRoles::<T>::get(&new_role, &user).unwrap(), Error::<T>::AccessDenied);

			Ok(())
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		#[pallet::call_index(1)]
		pub fn assign(
			origin: OriginFor<T>,
			user: T::AccountId,
			new_role: [u8; 32],
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::assign_role(user.clone(), new_role)?;

			Ok(())
		}

		#[pallet::weight(0)]
		#[pallet::call_index(2)]
		pub fn revoke(
			origin: OriginFor<T>,
			user: T::AccountId,
			new_role: [u8; 32],
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::revoke_role(user.clone(), new_role)?;

			Ok(())
		}

		#[pallet::weight(0)]
		#[pallet::call_index(3)]
		pub fn has_role(
			origin: OriginFor<T>,
			user: T::AccountId,
			new_role: [u8; 32],
		) -> DispatchResult {
			let _sender = ensure_signed(origin.clone())?;

			Self::validate_role(user.clone(), new_role)?;

			Ok(())
		}
	}
}
