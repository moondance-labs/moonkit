// Copyright Moonsong Labs
// This file is part of Moonkit.

// Moonkit is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonkit is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonkit.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

pub use crate::weights::WeightInfo;
use cumulus_primitives_core::relay_chain::BlockNumber as RelayBlockNumber;
use cumulus_primitives_core::PersistedValidationData;
use frame_support::inherent::IsFatalError;
use frame_support::pallet;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sp_core::Get;
use sp_core::H256;
use sp_runtime::RuntimeString;
use sp_std::collections::vec_deque::VecDeque;

#[cfg(any(test, feature = "runtime-benchmarks"))]
mod benchmarks;
pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[pallet]
pub mod pallet {
	use super::*;

	/// The InherentIdentifier "relay storage root"
	pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"relsroot";

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	/// Configuration trait of this pallet.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type GetPersistedValidationData: Get<PersistedValidationData>;
		#[pallet::constant]
		type MaxStorageRoots: Get<u32>;
		/// Weight info
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		RequestCounterOverflowed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		RequestExpirationExecuted { id: u8 },
	}

	#[derive(Encode)]
	#[cfg_attr(feature = "std", derive(Debug, Decode))]
	pub enum InherentError {
		Other(RuntimeString),
	}

	impl IsFatalError for InherentError {
		fn is_fatal_error(&self) -> bool {
			match *self {
				InherentError::Other(_) => true,
			}
		}
	}

	impl InherentError {
		/// Try to create an instance ouf of the given identifier and data.
		#[cfg(feature = "std")]
		pub fn try_from(id: &InherentIdentifier, data: &[u8]) -> Option<Self> {
			if id == &INHERENT_IDENTIFIER {
				<InherentError as parity_scale_codec::Decode>::decode(&mut &data[..]).ok()
			} else {
				None
			}
		}
	}

	/// Ensures the mandatory inherent was included in the block
	#[pallet::storage]
	#[pallet::getter(fn inherent_included)]
	pub(crate) type InherentIncluded<T: Config> = StorageValue<_, ()>;

	/// Map of relay block number to relay storage root
	#[pallet::storage]
	pub type RelayStorageRoot<T: Config> =
		StorageMap<_, Twox64Concat, RelayBlockNumber, H256, OptionQuery>;

	/// List of all the keys in `RelayStorageRoot`.
	/// Used to remove the oldest key without having to iterate over all of them.
	#[pallet::storage]
	pub type RelayStorageRootKeys<T: Config> =
		StorageValue<_, VecDeque<RelayBlockNumber>, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Populates `RelayStorageRoot` using this block's `PersistedValidationData`.
		#[pallet::call_index(0)]
		#[pallet::weight((
			T::WeightInfo::set_relay_storage_root(),
			DispatchClass::Mandatory
		))]
		pub fn set_relay_storage_root(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			let validation_data = T::GetPersistedValidationData::get();

			if <RelayStorageRoot<T>>::contains_key(validation_data.relay_parent_number) {
				<InherentIncluded<T>>::put(());
				return Ok(Pays::No.into());
			}

			<RelayStorageRoot<T>>::insert(
				validation_data.relay_parent_number,
				validation_data.relay_parent_storage_root,
			);

			let mut keys = <RelayStorageRootKeys<T>>::get();
			keys.push_back(validation_data.relay_parent_number);
			// Delete the oldest stored root if the total number is greater than MaxStorageRoots
			if u32::try_from(keys.len()).unwrap() > T::MaxStorageRoots::get() {
				let first_key = keys.pop_front().unwrap();
				<RelayStorageRoot<T>>::remove(first_key);
			}

			<RelayStorageRootKeys<T>>::put(keys);
			<InherentIncluded<T>>::put(());
			Ok(Pays::No.into())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn is_inherent_required(_: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			// Return Ok(Some(_)) unconditionally because this inherent is required in every block
			Ok(Some(InherentError::Other(
				sp_runtime::RuntimeString::Borrowed("Inherent required to set relay storage roots"),
			)))
		}

		// The empty-payload inherent extrinsic.
		fn create_inherent(_data: &InherentData) -> Option<Self::Call> {
			Some(Call::set_relay_storage_root {})
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::set_relay_storage_root { .. })
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
			// 1 read and 1 write in on_finalize
			T::DbWeight::get().reads_writes(1, 1)
		}
		fn on_finalize(_now: BlockNumberFor<T>) {
			// Ensure the mandatory inherent was included in the block or the block is invalid
			assert!(
				<InherentIncluded<T>>::take().is_some(),
				"Mandatory pallet_relay_storage_roots inherent not included; InherentIncluded storage item is empty"
			);
		}
	}
}
