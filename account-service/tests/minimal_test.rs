#[cfg(test)]
mod tests {
    use account_service::InMemoryAccountRepository;
    use common::decimal::{Quantity, dec};
    use common::model::account::{Account, Balance};
    use uuid::Uuid;
    
    #[test]
    fn test_balance_operations() {
        let account_id = Uuid::new_v4();
        let mut balance = Balance::new(account_id, "BTC".to_string());
        
        // Test initial state
        assert_eq!(balance.total, Quantity::ZERO);
        assert_eq!(balance.available, Quantity::ZERO);
        assert_eq!(balance.locked, Quantity::ZERO);
        
        // Test deposit
        balance.deposit(dec!(100));
        assert_eq!(balance.total, dec!(100));
        assert_eq!(balance.available, dec!(100));
        assert_eq!(balance.locked, Quantity::ZERO);
        
        // Test lock funds
        balance.lock(dec!(30)).unwrap();
        assert_eq!(balance.total, dec!(100));
        assert_eq!(balance.available, dec!(70));
        assert_eq!(balance.locked, dec!(30));
        
        // Test withdraw
        balance.withdraw(dec!(20)).unwrap();
        assert_eq!(balance.total, dec!(80));
        assert_eq!(balance.available, dec!(50));
        assert_eq!(balance.locked, dec!(30));
        
        // Test insufficient funds
        let result = balance.withdraw(dec!(60));
        assert!(result.is_err());
        
        // Test unlock
        balance.unlock(dec!(10));
        assert_eq!(balance.total, dec!(80));
        assert_eq!(balance.available, dec!(60));
        assert_eq!(balance.locked, dec!(20));
    }
    
    #[test]
    fn test_in_memory_repository() {
        let repo = InMemoryAccountRepository::new();
        
        // Test accounts map is initially empty
        assert!(repo.accounts.is_empty());
        assert!(repo.balances.is_empty());
        
        // Add test account
        let account_id = Uuid::new_v4();
        let account = Account {
            id: account_id,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        repo.accounts.insert(account_id, account);
        
        // Add test balance
        let balance = Balance::new(account_id, "USD".to_string());
        repo.balances.insert((account_id, "USD".to_string()), balance);
        
        // Verify items were added
        assert_eq!(repo.accounts.len(), 1);
        assert_eq!(repo.balances.len(), 1);
        
        // Verify account can be retrieved
        let retrieved = repo.accounts.get(&account_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, account_id);
        
        // Verify balance can be retrieved
        let balance_key = (account_id, "USD".to_string());
        let retrieved = repo.balances.get(&balance_key);
        assert!(retrieved.is_some());
        let bal = retrieved.unwrap();
        assert_eq!(bal.account_id, account_id);
        assert_eq!(bal.asset, "USD");
    }
}