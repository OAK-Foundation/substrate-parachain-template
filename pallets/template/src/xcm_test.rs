#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};

use sp_std::prelude::*;
use frame_support::RuntimeDebug;
use cumulus_primitives_core::ParaId;

#[derive(Encode, Decode, RuntimeDebug)]
pub enum SystemCall {
    #[codec(index = 8)]
    RemarkWithEvent(Vec<u8>),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum AutomationTimeCall {
    #[codec(index = 0)]
    ScheduleNotifyTask(Vec<u8>, Vec<u64>, Vec<u8>),
    #[codec(index = 2)]
    ScheduleXcmpTask(Vec<u8>, Vec<u64>, ParaId, Vec<u8>, u64),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum TemplateXCMCall<T: frame_system::Config, BalanceOf> {
    #[codec(index = 1)]
    ForceSendBalance(T::AccountId, T::AccountId, BalanceOf),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum NeuChainCall {
    #[codec(index = 0)]
    System(SystemCall),
    #[codec(index = 60)]
    AutomationTime(AutomationTimeCall),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum TestChainCall<T: frame_system::Config, BalanceOf> {
    #[codec(index = 0)]
    System(SystemCall),
    #[codec(index = 60)]
    TemplateXCMCall(TemplateXCMCall<T, BalanceOf>),
}

pub struct OakChainCallBuilder;

impl OakChainCallBuilder {
    pub fn remark_with_event<T: frame_system::Config, BalanceOf>(message: Vec<u8>) -> NeuChainCall {
        NeuChainCall::System(SystemCall::RemarkWithEvent(message))
    }
}
