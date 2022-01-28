use futures_executor::block_on;
use maplit::hashmap;
use matches::assert_matches;
use tuf::crypto::{Ed25519PrivateKey, HashAlgorithm, PrivateKey};
use tuf::interchange::Json;
use tuf::metadata::{
    Delegation, Delegations, MetadataDescription, MetadataPath, Role, TargetPath,
    TargetsMetadataBuilder,
};
use tuf::repo_builder::RepoBuilder;
use tuf::repository::EphemeralRepository;
use tuf::Database;
use tuf::Error;

const ED25519_1_PK8: &[u8] = include_bytes!("./ed25519/ed25519-1.pk8.der");
const ED25519_2_PK8: &[u8] = include_bytes!("./ed25519/ed25519-2.pk8.der");
const ED25519_3_PK8: &[u8] = include_bytes!("./ed25519/ed25519-3.pk8.der");
const ED25519_4_PK8: &[u8] = include_bytes!("./ed25519/ed25519-4.pk8.der");
const ED25519_5_PK8: &[u8] = include_bytes!("./ed25519/ed25519-5.pk8.der");
const ED25519_6_PK8: &[u8] = include_bytes!("./ed25519/ed25519-6.pk8.der");

#[test]
fn simple_delegation() {
    block_on(async {
        let root_key = Ed25519PrivateKey::from_pkcs8(ED25519_1_PK8).unwrap();
        let snapshot_key = Ed25519PrivateKey::from_pkcs8(ED25519_2_PK8).unwrap();
        let targets_key = Ed25519PrivateKey::from_pkcs8(ED25519_3_PK8).unwrap();
        let timestamp_key = Ed25519PrivateKey::from_pkcs8(ED25519_4_PK8).unwrap();
        let delegation_key = Ed25519PrivateKey::from_pkcs8(ED25519_5_PK8).unwrap();

        let delegations = Delegations::new(
            hashmap! { delegation_key.public().key_id().clone() => delegation_key.public().clone() },
            vec![Delegation::new(
                MetadataPath::new("delegation").unwrap(),
                false,
                1,
                vec![delegation_key.public().key_id().clone()]
                    .iter()
                    .cloned()
                    .collect(),
                vec![TargetPath::new("foo").unwrap()]
                    .iter()
                    .cloned()
                    .collect(),
            )
            .unwrap()],
        )
        .unwrap();

        let mut repo = EphemeralRepository::new();
        let metadata = RepoBuilder::create(&mut repo)
            .trusted_root_keys(&[&root_key])
            .trusted_snapshot_keys(&[&snapshot_key])
            .trusted_targets_keys(&[&targets_key])
            .trusted_timestamp_keys(&[&timestamp_key])
            .stage_root()
            .unwrap()
            .stage_targets_with_builder(|builder| builder.delegations(delegations))
            .unwrap()
            .stage_snapshot_with_builder(|builder| {
                builder.insert_metadata_description(
                    MetadataPath::new("delegation").unwrap(),
                    MetadataDescription::from_slice(&[0u8], 1, &[HashAlgorithm::Sha256]).unwrap(),
                )
            })
            .unwrap()
            .commit()
            .await
            .unwrap();

        let mut tuf = Database::<Json>::from_trusted_metadata(&metadata).unwrap();

        //// build the targets ////
        //// build the delegation ////
        let target_file: &[u8] = b"bar";
        let delegation = TargetsMetadataBuilder::new()
            .insert_target_from_slice(
                TargetPath::new("foo").unwrap(),
                target_file,
                &[HashAlgorithm::Sha256],
            )
            .unwrap()
            .signed::<Json>(&delegation_key)
            .unwrap();
        let raw_delegation = delegation.to_raw().unwrap();

        tuf.update_delegation(
            &MetadataPath::from_role(&Role::Targets),
            &MetadataPath::new("delegation").unwrap(),
            &raw_delegation,
        )
        .unwrap();

        assert!(tuf
            .target_description(&TargetPath::new("foo").unwrap())
            .is_ok());
    })
}

#[test]
fn nested_delegation() {
    block_on(async {
        let root_key = Ed25519PrivateKey::from_pkcs8(ED25519_1_PK8).unwrap();
        let snapshot_key = Ed25519PrivateKey::from_pkcs8(ED25519_2_PK8).unwrap();
        let targets_key = Ed25519PrivateKey::from_pkcs8(ED25519_3_PK8).unwrap();
        let timestamp_key = Ed25519PrivateKey::from_pkcs8(ED25519_4_PK8).unwrap();
        let delegation_a_key = Ed25519PrivateKey::from_pkcs8(ED25519_5_PK8).unwrap();
        let delegation_b_key = Ed25519PrivateKey::from_pkcs8(ED25519_6_PK8).unwrap();

        let delegations = Delegations::new(
            hashmap! {
                delegation_a_key.public().key_id().clone() => delegation_a_key.public().clone(),
            },
            vec![Delegation::new(
                MetadataPath::new("delegation-a").unwrap(),
                false,
                1,
                vec![delegation_a_key.public().key_id().clone()]
                    .iter()
                    .cloned()
                    .collect(),
                vec![TargetPath::new("foo").unwrap()]
                    .iter()
                    .cloned()
                    .collect(),
            )
            .unwrap()],
        )
        .unwrap();

        let mut repo = EphemeralRepository::new();
        let metadata = RepoBuilder::create(&mut repo)
            .trusted_root_keys(&[&root_key])
            .trusted_snapshot_keys(&[&snapshot_key])
            .trusted_targets_keys(&[&targets_key])
            .trusted_timestamp_keys(&[&timestamp_key])
            .stage_root()
            .unwrap()
            .stage_targets_with_builder(|builder| builder.delegations(delegations))
            .unwrap()
            .stage_snapshot_with_builder(|builder| {
                builder
                    .insert_metadata_description(
                        MetadataPath::new("delegation-a").unwrap(),
                        MetadataDescription::from_slice(&[0u8], 1, &[HashAlgorithm::Sha256])
                            .unwrap(),
                    )
                    .insert_metadata_description(
                        MetadataPath::new("delegation-b").unwrap(),
                        MetadataDescription::from_slice(&[0u8], 1, &[HashAlgorithm::Sha256])
                            .unwrap(),
                    )
            })
            .unwrap()
            .commit()
            .await
            .unwrap();

        let mut tuf = Database::<Json>::from_trusted_metadata(&metadata).unwrap();

        //// build delegation A ////

        let delegations = Delegations::new(
        hashmap! { delegation_b_key.public().key_id().clone() => delegation_b_key.public().clone() },
        vec![Delegation::new(
            MetadataPath::new("delegation-b").unwrap(),
            false,
            1,
            vec![delegation_b_key.public().key_id().clone()].iter().cloned().collect(),
            vec![TargetPath::new("foo").unwrap()].iter().cloned().collect(),
        )
        .unwrap()],
    )
    .unwrap();

        let delegation = TargetsMetadataBuilder::new()
            .delegations(delegations)
            .signed::<Json>(&delegation_a_key)
            .unwrap();
        let raw_delegation = delegation.to_raw().unwrap();

        tuf.update_delegation(
            &MetadataPath::from_role(&Role::Targets),
            &MetadataPath::new("delegation-a").unwrap(),
            &raw_delegation,
        )
        .unwrap();

        //// build delegation B ////

        let target_file: &[u8] = b"bar";

        let delegation = TargetsMetadataBuilder::new()
            .insert_target_from_slice(
                TargetPath::new("foo").unwrap(),
                target_file,
                &[HashAlgorithm::Sha256],
            )
            .unwrap()
            .signed::<Json>(&delegation_b_key)
            .unwrap();
        let raw_delegation = delegation.to_raw().unwrap();

        tuf.update_delegation(
            &MetadataPath::new("delegation-a").unwrap(),
            &MetadataPath::new("delegation-b").unwrap(),
            &raw_delegation,
        )
        .unwrap();

        assert!(tuf
            .target_description(&TargetPath::new("foo").unwrap())
            .is_ok());
    })
}

#[test]
fn rejects_bad_delegation_signatures() {
    block_on(async {
        let root_key = Ed25519PrivateKey::from_pkcs8(ED25519_1_PK8).unwrap();
        let snapshot_key = Ed25519PrivateKey::from_pkcs8(ED25519_2_PK8).unwrap();
        let targets_key = Ed25519PrivateKey::from_pkcs8(ED25519_3_PK8).unwrap();
        let timestamp_key = Ed25519PrivateKey::from_pkcs8(ED25519_4_PK8).unwrap();
        let delegation_key = Ed25519PrivateKey::from_pkcs8(ED25519_5_PK8).unwrap();
        let bad_delegation_key = Ed25519PrivateKey::from_pkcs8(ED25519_6_PK8).unwrap();

        let delegations = Delegations::new(
            hashmap! { delegation_key.public().key_id().clone() => delegation_key.public().clone() },
            vec![Delegation::new(
                MetadataPath::new("delegation").unwrap(),
                false,
                1,
                vec![delegation_key.public().key_id().clone()]
                    .iter()
                    .cloned()
                    .collect(),
                vec![TargetPath::new("foo").unwrap()]
                    .iter()
                    .cloned()
                    .collect(),
            )
            .unwrap()],
        )
        .unwrap();

        let mut repo = EphemeralRepository::new();
        let metadata = RepoBuilder::create(&mut repo)
            .trusted_root_keys(&[&root_key])
            .trusted_snapshot_keys(&[&snapshot_key])
            .trusted_targets_keys(&[&targets_key])
            .trusted_timestamp_keys(&[&timestamp_key])
            .stage_root()
            .unwrap()
            .stage_targets_with_builder(|builder| builder.delegations(delegations))
            .unwrap()
            .stage_snapshot_with_builder(|builder| {
                builder.insert_metadata_description(
                    MetadataPath::new("delegation").unwrap(),
                    MetadataDescription::from_slice(&[0u8], 1, &[HashAlgorithm::Sha256]).unwrap(),
                )
            })
            .unwrap()
            .commit()
            .await
            .unwrap();

        let mut tuf = Database::<Json>::from_trusted_metadata(&metadata).unwrap();

        //// build the delegation ////
        let target_file: &[u8] = b"bar";
        let delegation = TargetsMetadataBuilder::new()
            .insert_target_from_slice(
                TargetPath::new("foo").unwrap(),
                target_file,
                &[HashAlgorithm::Sha256],
            )
            .unwrap()
            .signed::<Json>(&bad_delegation_key)
            .unwrap();
        let raw_delegation = delegation.to_raw().unwrap();

        assert_matches!(
            tuf.update_delegation(
                &MetadataPath::from_role(&Role::Targets),
                &MetadataPath::new("delegation").unwrap(),
                &raw_delegation
            ),
            Err(Error::VerificationFailure(_))
        );

        assert_matches!(
            tuf.target_description(&TargetPath::new("foo").unwrap()),
            Err(Error::TargetUnavailable)
        );
    })
}

#[test]
fn diamond_delegation() {
    block_on(async {
        let etc_key = Ed25519PrivateKey::from_pkcs8(ED25519_1_PK8).unwrap();
        let targets_key = Ed25519PrivateKey::from_pkcs8(ED25519_2_PK8).unwrap();
        let delegation_a_key = Ed25519PrivateKey::from_pkcs8(ED25519_3_PK8).unwrap();
        let delegation_b_key = Ed25519PrivateKey::from_pkcs8(ED25519_4_PK8).unwrap();
        let delegation_c_key = Ed25519PrivateKey::from_pkcs8(ED25519_5_PK8).unwrap();

        // Given delegations a, b, and c, targets delegates "foo" to delegation-a and "bar" to
        // delegation-b.
        //
        //             targets
        //              /  \
        //   delegation-a  delegation-b
        //              \  /
        //          delegation-c
        //
        // if delegation-a delegates "foo" to delegation-c, and
        //    delegation-b delegates "bar" to delegation-c, but
        //    delegation-b's signature is invalid, then delegation-c
        // can contain target "bar" which is unaccessible and target "foo" which is.
        //
        // Verify tuf::Database handles this situation correctly.

        let delegations = Delegations::new(
            hashmap! {
                delegation_a_key.public().key_id().clone() => delegation_a_key.public().clone(),
                delegation_b_key.public().key_id().clone() => delegation_b_key.public().clone(),
            },
            vec![
                Delegation::new(
                    MetadataPath::new("delegation-a").unwrap(),
                    false,
                    1,
                    vec![delegation_a_key.public().key_id().clone()]
                        .iter()
                        .cloned()
                        .collect(),
                    vec![TargetPath::new("foo").unwrap()]
                        .iter()
                        .cloned()
                        .collect(),
                )
                .unwrap(),
                Delegation::new(
                    MetadataPath::new("delegation-b").unwrap(),
                    false,
                    1,
                    vec![delegation_b_key.public().key_id().clone()]
                        .iter()
                        .cloned()
                        .collect(),
                    vec![TargetPath::new("bar").unwrap()]
                        .iter()
                        .cloned()
                        .collect(),
                )
                .unwrap(),
            ],
        )
        .unwrap();

        let mut repo = EphemeralRepository::new();
        let metadata = RepoBuilder::create(&mut repo)
            .trusted_root_keys(&[&etc_key])
            .trusted_snapshot_keys(&[&etc_key])
            .trusted_targets_keys(&[&targets_key])
            .trusted_timestamp_keys(&[&etc_key])
            .stage_root()
            .unwrap()
            .stage_targets_with_builder(|builder| builder.delegations(delegations))
            .unwrap()
            .stage_snapshot_with_builder(|builder| {
                builder
                    .insert_metadata_description(
                        MetadataPath::new("delegation-a").unwrap(),
                        MetadataDescription::from_slice(&[0u8], 1, &[HashAlgorithm::Sha256])
                            .unwrap(),
                    )
                    .insert_metadata_description(
                        MetadataPath::new("delegation-b").unwrap(),
                        MetadataDescription::from_slice(&[0u8], 1, &[HashAlgorithm::Sha256])
                            .unwrap(),
                    )
                    .insert_metadata_description(
                        MetadataPath::new("delegation-c").unwrap(),
                        MetadataDescription::from_slice(&[0u8], 1, &[HashAlgorithm::Sha256])
                            .unwrap(),
                    )
            })
            .unwrap()
            .commit()
            .await
            .unwrap();

        let mut tuf = Database::<Json>::from_trusted_metadata(&metadata).unwrap();

        //// build delegation A ////

        let delegations = Delegations::new(
        hashmap! { delegation_c_key.public().key_id().clone() => delegation_c_key.public().clone() },
        vec![Delegation::new(
            MetadataPath::new("delegation-c").unwrap(),
            false,
            1,
            vec![delegation_c_key.public().key_id().clone()].iter().cloned().collect(),
            vec![TargetPath::new("foo").unwrap()].iter().cloned().collect(),
        )
        .unwrap()],
    )
    .unwrap();

        let delegation = TargetsMetadataBuilder::new()
            .delegations(delegations)
            .signed::<Json>(&delegation_a_key)
            .unwrap();
        let raw_delegation = delegation.to_raw().unwrap();

        tuf.update_delegation(
            &MetadataPath::from_role(&Role::Targets),
            &MetadataPath::new("delegation-a").unwrap(),
            &raw_delegation,
        )
        .unwrap();

        //// build delegation B ////

        let delegations = Delegations::new(
        hashmap! { delegation_c_key.public().key_id().clone() => delegation_c_key.public().clone() },
        vec![Delegation::new(
            MetadataPath::new("delegation-c").unwrap(),
            false,
            1,
            // oops, wrong key.
            vec![delegation_b_key.public().key_id().clone()].iter().cloned().collect(),
            vec![TargetPath::new("bar").unwrap()].iter().cloned().collect(),
        )
        .unwrap()],
    )
    .unwrap();

        let delegation = TargetsMetadataBuilder::new()
            .delegations(delegations)
            .signed::<Json>(&delegation_b_key)
            .unwrap();
        let raw_delegation = delegation.to_raw().unwrap();

        tuf.update_delegation(
            &MetadataPath::from_role(&Role::Targets),
            &MetadataPath::new("delegation-b").unwrap(),
            &raw_delegation,
        )
        .unwrap();

        //// build delegation C ////

        let foo_target_file: &[u8] = b"foo contents";
        let bar_target_file: &[u8] = b"bar contents";

        let delegation = TargetsMetadataBuilder::new()
            .insert_target_from_slice(
                TargetPath::new("foo").unwrap(),
                foo_target_file,
                &[HashAlgorithm::Sha256],
            )
            .unwrap()
            .insert_target_from_slice(
                TargetPath::new("bar").unwrap(),
                bar_target_file,
                &[HashAlgorithm::Sha256],
            )
            .unwrap()
            .signed::<Json>(&delegation_c_key)
            .unwrap();
        let raw_delegation = delegation.to_raw().unwrap();

        //// Verify delegation-c is valid, but only when updated through delegation-a.

        tuf.update_delegation(
            &MetadataPath::new("delegation-a").unwrap(),
            &MetadataPath::new("delegation-c").unwrap(),
            &raw_delegation,
        )
        .unwrap();

        assert_matches!(
            tuf.update_delegation(
                &MetadataPath::new("delegation-b").unwrap(),
                &MetadataPath::new("delegation-c").unwrap(),
                &raw_delegation
            ),
            Err(Error::VerificationFailure(_))
        );

        assert!(tuf
            .target_description(&TargetPath::new("foo").unwrap())
            .is_ok());

        assert_matches!(
            tuf.target_description(&TargetPath::new("bar").unwrap()),
            Err(Error::TargetUnavailable)
        );
    })
}
