// Tests to be written here

use crate::{Error, mock::*};
use super::*;
use frame_support::{assert_ok, assert_noop};
// use frame_system::Origin;

// test cases for create_claim
#[test]
fn create_claim_works() {
    new_test_ext().execute_with(|| {
        let claim = vec![0, 1];
        assert_ok!(PoeModule::create_claim(Origin::signed(1),claim.clone()));
        assert_eq!(Proofs::<Test>::get(&claim),(1,system::Module::<Test>::block_number()));
    })
}
#[test]
fn create_claim_failed_when_claim_already_exist(){
    new_test_ext().execute_with(||{
        let claim = vec![0,1];
        PoeModule::create_claim(Origin::signed(1),claim.clone());
        assert_noop!(PoeModule::create_claim(Origin::signed(1),claim.clone()),Error::<Test>::ProofAlreadyExist);
    })
}
#[test]
fn create_claim_failed_when_claim_data_too_long(){
    new_test_ext().execute_with(||{
        let claim = vec![0,1,3,4,5,6,7];
        assert_noop!(PoeModule::create_claim(Origin::signed(1),claim.clone()),Error::<Test>::ProofTooLong);
    })
}
#[test]
fn revoke_claim(){
    new_test_ext().execute_with(||{
        let claim = vec![0,1];
        let _=PoeModule::create_claim(Origin::signed(1),claim.clone());
        assert_ok!(PoeModule::revoke_claim(Origin::signed(1),claim.clone()));
    })
}
#[test]
fn revoke_claim_failed_when_claim_not_exist(){
    new_test_ext().execute_with(||{
        let claim = vec![0,1];
        assert_noop!(PoeModule::revoke_claim(Origin::signed(1),claim.clone()),Error::<Test>::ClaimNotExist);
    })
}
#[test]
fn revoke_claim_failed_when_owner_not_match(){
    new_test_ext().execute_with(||{
        let claim = vec![0,1];
        let _=PoeModule::create_claim(Origin::signed(1),claim.clone());
        assert_noop!(PoeModule::revoke_claim(Origin::signed(2),claim.clone()),Error::<Test>::NotClaimOwner);
    })
}