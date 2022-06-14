#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>

use cumulus_pallet_xcm::{ensure_sibling_para, Origin as CumulusOrigin};
use cumulus_primitives_core::ParaId;
use frame_system::Config as SystemConfig;
use sp_std::prelude::*;
use xcm::latest::prelude::*;

pub use pallet::*;
use oak_xcm::{XcmInstructionGenerator};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod xcm_test;

#[frame_support::pallet]
pub mod pallet {
	use xcm_executor::traits::WeightBounds;

use super::*;
	use frame_support::{pallet_prelude::*, traits::{ExistenceRequirement, Currency}};
	use frame_system::pallet_prelude::*;
	use log::info;
	use sp_runtime::traits::Convert;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Origin: From<<Self as SystemConfig>::Origin>
			+ Into<Result<CumulusOrigin, <Self as Config>::Origin>>;

		type Call: From<Call<Self>> + Encode;

		type XcmSender: SendXcm;
		type XcmExecutor: ExecuteXcm<<Self as pallet::Config>::Call>;
		// type WeightInfo: pallet_automation_time::WeightInfo;
		
		type Weigher: WeightBounds<<Self as pallet::Config>::Call>;
		
		type AccountIdToMultiLocation: Convert<Self::AccountId, MultiLocation>;
		type AccountIdToU8Vec: Convert<Self::AccountId, [u8; 32]>;
		type OakXcmInstructionGenerator: XcmInstructionGenerator<Self>;
		type Currency: Currency<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// [call]
		CallSent(Vec<u8>),
		/// [error, paraId, call]
		ErrorSendingCall(SendError, ParaId, Vec<u8>),
	}

	#[pallet::error]
	pub enum Error<T> {
		// TODO: expand into XcmExecutionFailed(XcmError) after https://github.com/paritytech/substrate/pull/10242 done
		/// XCM execution failed.
		XcmExecutionFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn send_remark_with_event(origin: OriginFor<T>, para: ParaId, message: Vec<u8>) -> DispatchResult {
			ensure_root(origin)?;

			let call_name = b"remark_with_event".to_vec();
			let remark = xcm_test::OakChainCallBuilder::remark_with_event::<T, BalanceOf<T>>(message);

			match T::XcmSender::send_xcm(
				(1, Junction::Parachain(para.into())),
				Xcm(vec![Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: 10_000_000_000,
					call: remark.encode().into(),
				}]),
			) {
				Ok(()) => {
					Self::deposit_event(Event::CallSent(call_name));
				},
				Err(e) => {
					Self::deposit_event(Event::ErrorSendingCall(e, para, call_name));
				}
			};
			Ok(())
		}

		/**
		 * This function wraps the currency transfer private function that can move tokens from wallet to wallet. 
		 * The intention is for this function to be called by the `Transact` XCM instruction when Turing calls back to this chain.
		 * While we are using transfer, this is just an example and any private function can be used to substitute.
		 *
		 * By calling `ensure_sibling_para`, we can ensure that only sibling parachains will be able to call and returns the para ID.
		 * Using this parachain ID, the user can make sure that the para ID is whitelisted.
		 */
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn force_send_balance(origin: OriginFor<T>, source: T::AccountId, dest: T::AccountId, value: BalanceOf<T>) -> DispatchResult {
			let origin_para_id: ParaId = ensure_sibling_para(<T as Config>::Origin::from(origin))?;
			info!("Force sending balance, source: {:?}, dest: {:?}, value: {:?}, origin_para_id: {:?}", source, dest, value, origin_para_id);
			<T as Config>::Currency::transfer(
				&source,
				&dest,
				value,
				ExistenceRequirement::AllowDeath,
			)?;
			Ok(())
		}

		/**
		 * This function implements XCM call to OAK with the OAK XCM crate. It uses the `force_send_balance` extrinsic above.
		 * We can create an XCMP instruction with that call wrapped in the instructions to be sent back to this chain.
		 * This implementation withdraws assets for the fee from the sovereign account of this chain on Turing.
		 * Therefore, TUR tokens must be available for this sovereign account on the Turing chain. 
		 */
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn send_schedule_xcmp_with_crate(
			origin: OriginFor<T>,
			para: ParaId,
			para_response_location: ParaId,
			provided_id: Vec<u8>,
			execution_times: Vec<u64>,
			source: T::AccountId,
			dest: T::AccountId,
			value: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let call_name = b"automation_time_schedule_xcmp_with_crate".to_vec();
			let inner_call = <T as Config>::Call::from(Call::<T>::force_send_balance { source: source.clone(), dest, value })
				.encode()
				.into();
			let transact_instruction =
				T::OakXcmInstructionGenerator::create_schedule_xcmp_instruction(provided_id, execution_times, para_response_location, inner_call);
			let asset = MultiAsset {
				id: Concrete(MultiLocation::here()),
				fun: Fungibility::Fungible(7_000_000_000),
			};

			let xcm_instruction_set = T::OakXcmInstructionGenerator::create_xcm_instruction_set(asset, transact_instruction, source);

			match T::XcmSender::send_xcm(
				(1, Junction::Parachain(para.into())),
				xcm_instruction_set,
			) {
				Ok(()) => {
					Self::deposit_event(Event::CallSent(call_name));
				},
				Err(e) => {
					Self::deposit_event(Event::ErrorSendingCall(e, para, call_name));
				}
			};
			Ok(())
		}
	}
}
