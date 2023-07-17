// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;

use codec::{Decode, Encode};
use frame_support::{dispatch::DispatchResult, traits::Task};
// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;
use sp_runtime::DispatchError;

#[derive(Clone, PartialEq, Eq, Encode, Decode)]
pub enum Never {}

#[derive(Clone, PartialEq, Eq, Encode, Decode)]
pub enum ExampleTask<T: Config> {
	Increment,
	Decrement,
	_Phantom(PhantomData<T>, Never),
}

impl<T: Config> Task for ExampleTask<T> {
	type Enumeration = std::vec::IntoIter<ExampleTask<T>>;

	const TASK_INDEX: usize = 0;

	fn enumerate() -> Self::Enumeration {
		vec![ExampleTask::Increment, ExampleTask::Decrement].into_iter()
	}

	fn is_valid(&self) -> bool {
		let value = Value::<T>::get().unwrap();
		match self {
			ExampleTask::Increment => value < 255,
			ExampleTask::Decrement => value > 0,
			ExampleTask::_Phantom(_, _) => unreachable!(),
		}
	}

	fn run(&self) -> Result<(), DispatchError> {
		match self {
			ExampleTask::Increment => {
				// Increment the value and emit an event
				let new_val = Value::<T>::get().unwrap().checked_add(1).ok_or("Value overflow")?;
				Value::<T>::put(new_val);
				Pallet::<T>::deposit_event(Event::Incremented { new_val });
			},
			ExampleTask::Decrement => {
				// Decrement the value and emit an event
				let new_val = Value::<T>::get().unwrap().checked_sub(1).ok_or("Value underflow")?;
				Value::<T>::put(new_val);
				Pallet::<T>::deposit_event(Event::Decremented { new_val });
			},
			ExampleTask::_Phantom(_, _) => unreachable!(),
		}
		Ok(())
	}
}

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn value)]
	pub type Value<T: Config> = StorageValue<_, u8>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		pub fn increment(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			// Increment the value and emit an event
			let new_val = Value::<T>::get().unwrap().checked_add(1).ok_or("Value overflow")?;
			Value::<T>::put(new_val);
			Self::deposit_event(Event::Incremented { new_val });

			Ok(())
		}

		pub fn decrement(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;

			// Decrement the value and emit an event
			let new_val = Value::<T>::get().unwrap().checked_sub(1).ok_or("Value underflow")?;
			Value::<T>::put(new_val);
			Self::deposit_event(Event::Decremented { new_val });

			Ok(())
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Incremented { new_val: u8 },
		Decremented { new_val: u8 },
	}
}