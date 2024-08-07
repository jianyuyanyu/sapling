/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

use commit_cloud::sql::builder::SqlCommitCloudBuilder;
use commit_cloud::sql::ops::Insert;
use fbinit::FacebookInit;
use mononoke_types::Timestamp;
use sql_construct::SqlConstruct;

#[fbinit::test]
async fn test_versions(_fb: FacebookInit) -> anyhow::Result<()> {
    use commit_cloud::references::versions::WorkspaceVersion;
    use commit_cloud::sql::ops::Get;
    use commit_cloud::sql::versions_ops::get_version_by_prefix;
    let sql = SqlCommitCloudBuilder::with_sqlite_in_memory()?.new(false);
    let reponame = "test_repo".to_owned();
    let workspace = "user/testuser/default".to_owned();
    let initial_timestamp = Timestamp::now();
    let args = WorkspaceVersion {
        workspace: workspace.clone(),
        version: 1,
        timestamp: initial_timestamp,
        archived: false,
    };

    let mut txn = sql.connections.write_connection.start_transaction().await?;
    txn = sql
        .insert(txn, None, reponame.clone(), workspace.clone(), args.clone())
        .await?;
    txn.commit().await?;

    let res: Vec<WorkspaceVersion> = sql.get(reponame.clone(), workspace.clone()).await?;
    assert_eq!(vec![args.clone()], res);

    let res_prefix = get_version_by_prefix(
        &sql.connections,
        reponame.clone(),
        "user/testuser/".to_string(),
    )
    .await?;
    assert_eq!(vec![args], res_prefix);

    // Test version conflict
    let args2 = WorkspaceVersion {
        workspace: workspace.clone(),
        version: 2,
        timestamp: Timestamp::now(),
        archived: false,
    };

    txn = sql.connections.write_connection.start_transaction().await?;
    txn = sql
        .insert(
            txn,
            None,
            reponame.clone(),
            workspace.clone(),
            args2.clone(),
        )
        .await?;
    txn.commit().await?;
    let res2: Vec<WorkspaceVersion> = sql.get(reponame.clone(), workspace.clone()).await?;
    assert!(res2[0].timestamp > initial_timestamp);

    Ok(())
}
