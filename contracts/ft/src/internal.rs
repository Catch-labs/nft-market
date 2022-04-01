use crate::*;

#[macro_export]
macro_rules! require {
    ( $a:expr, $b:expr ) => {
        if !$a {
            env::panic($b.as_bytes());
        }
    };
}

pub(crate) fn assert_one_yocto() {
    require!(
        env::attached_deposit() == 1,
        "Require attached deposit of exactly 1 yoctoNEAR"
    );
}

pub(crate) fn assert_self() {
    require!(
        env::predecessor_account_id() == env::current_account_id(),
        "Private Method"
    );
}

impl Contract {
    pub(crate) fn internal_deposit(&mut self, account_id: &AccountId, amount: Balance) {
        let balance = self
            .accounts
            .get(&account_id)
            .unwrap_or_else(|| env::panic(b"The account is not registered"));

        if let Some(new_balance) = balance.checked_add(amount) {
            self.accounts.insert(&account_id, &new_balance);
        } else {
            env::panic(b"Balance overflow");
        }
    }

    pub(crate) fn internal_withdraw(&mut self, account_id: &AccountId, amount: Balance) {
        let balance = self
            .accounts
            .get(&account_id)
            .unwrap_or_else(|| env::panic(b"The account is not registered"));

        if let Some(new_balance) = balance.checked_sub(amount) {
            self.accounts.insert(&account_id, &new_balance);
        } else {
            env::panic(b"The account doesn't have enough balance");
        }
    }

    pub(crate) fn internal_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        amount: Balance,
        memo: Option<String>,
    ) {
        require!(
            sender_id != receiver_id,
            "Sender and receiver should be different"
        );

        require!(amount > 0, "The amount should be a positive number");

        self.internal_withdraw(sender_id, amount);
        self.internal_deposit(receiver_id, amount);
    }

    pub(crate) fn assert_owner(&self) {
        require!(
            env::predecessor_account_id() == self.owner_id,
            "It is a owner only method"
        );
    }
}
