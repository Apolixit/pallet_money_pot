# Pallet Money Pot



## Overview

The Money Pot pallet provides functionnality for creating pot for a receiver account.
There are two different money pot end type :
    * Amount target
        Balance will be send to the receiver when the target is reached.
    * Time target
        Balance will be send to the receiver when the target block is reached, whatever the money pot ballance (except if balance = 0, nothing will happen and the money pot
        is close)

Everyone can contribute to a money pot and balance account will be locked until amount is dispatch.

## Terminology

Here is a basic implementation of pallet_money_pot in runtime :
```rust
parameter_types! {
	pub const MaxMoneyPotCurrentlyOpen: u32 = 5;
	pub const MaxMoneyPotContributors: u32 = 1000;
	pub const MinContribution: u32 = 5;
	pub const StepContribution: u32 = 5;
}

impl pallet_money_pot::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type Scheduler = Scheduler;
	type MaxMoneyPotCurrentlyOpen = MaxMoneyPotCurrentlyOpen;
	type MaxMoneyPotContributors = MaxMoneyPotContributors;
	type MinContribution = MinContribution;
	type StepContribution = StepContribution;
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type PalletsOrigin = OriginCaller;
	type Schedulable = Call;
	/// Max one year
	type MaxBlockNumberEndTime = ConstU32<{60 * 60 * 24 * 365 / SLOT_DURATION as u32}>;
}
```
* **Currency**: Balances pallet
* **Scheduler**: Scheduler pallet
* **MaxMoneyPotCurrentlyOpen**: The maximum of money pot which can be open at the same time for one account.
* **MaxMoneyPotContributors**: The maximum contributors for one money pot.
* **MinContribution**: The minimum balance contribution allowed for a money pot.
* **StepContribution**: The mandatory step balance to participate (for example 5 by 5).
* **ExistentialDeposit**: Same as Balances pallet - The minimum balance required at the end of the dispatch to keep the account alive.
* **MaxBlockNumberEndTime**: The maximum block number allowed to be dispatch. To avoid a money pot to be alive during infinite time.

## Interface

### Dispatchable Functions

These calls can be made from any externally held account capable of creating a signed extrinsic.

* `create_with_limit_amount` - Create a money pot with specified target amount. When amount is reached (>=), pot is send to receiver.
* `create_with_limit_block` - Create a money pot with specified block time. When block number is reached, pot is send to receiver.
* `add_balance` - Contribute to a money pot. The contributor account must a enought fund on free balance (doesn't check `ExistentialDeposit` at this point).
* `transfer_balance` - Require root autorisation, this function is called by Scheduler pallet or by root account.

### Front end POC

A [Front end](https://github.com/Apolixit/moneypot_blazor) developped with [ASP.NET Core Blazor](https://learn.microsoft.com/fr-fr/aspnet/core/blazor) and the [Ajuna.SDK](https://github.com/ajuna-network/Ajuna.SDK) is available if you want to try it out.