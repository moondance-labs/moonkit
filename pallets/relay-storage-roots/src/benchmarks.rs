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

#![cfg(feature = "runtime-benchmarks")]

//! Benchmarking
use crate::{
	Call, Config, InherentIncluded, Pallet, PersistedValidationData, RelayStorageRoot,
	RelayStorageRootKeys,
};
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite};
use frame_support::{assert_ok, traits::Get};
use frame_system::RawOrigin;
use sp_core::H256;

fn fill_relay_storage_roots<T: Config>() {
	for i in 0..T::MaxStorageRoots::get() {
		let relay_parent_number = i;
		let relay_parent_storage_root = H256::default();
		let validation_data: PersistedValidationData = PersistedValidationData {
			relay_parent_number,
			relay_parent_storage_root,
			..Default::default()
		};

		frame_support::storage::unhashed::put(b"MOCK_PERSISTED_VALIDATION_DATA", &validation_data);
		assert_ok!(Pallet::<T>::set_relay_storage_root(RawOrigin::None.into()));
	}

	assert!(
		u32::try_from(RelayStorageRootKeys::<T>::get().len()).unwrap() >= T::MaxStorageRoots::get()
	);
}

benchmarks! {
	// Benchmark for inherent included in every block
	set_relay_storage_root {
		// Worst case is when `RelayStorageRoot` has len of `MaxStorageRoots`
		fill_relay_storage_roots::<T>();

		let relay_parent_number = 1000;
		let relay_parent_storage_root = H256::default();
		let validation_data: PersistedValidationData = PersistedValidationData {
			relay_parent_number,
			relay_parent_storage_root,
			..Default::default()
		};
		frame_support::storage::unhashed::put(b"MOCK_PERSISTED_VALIDATION_DATA", &validation_data);
	}: _(RawOrigin::None)
	verify {
		// verify randomness result
		assert_eq!(
			RelayStorageRoot::<T>::get(
				relay_parent_number
			),
			Some(relay_parent_storage_root)
		);
		assert!(InherentIncluded::<T>::get().is_some());
	}
}

#[cfg(test)]
mod tests {
	use crate::mock::Test;
	use sp_io::TestExternalities;
	use sp_runtime::BuildStorage;

	pub fn new_test_ext() -> TestExternalities {
		let t = frame_system::GenesisConfig::<Test>::default()
			.build_storage()
			.unwrap();
		TestExternalities::new(t)
	}
}

impl_benchmark_test_suite!(
	Pallet,
	crate::benchmarks::tests::new_test_ext(),
	crate::mock::Test
);
