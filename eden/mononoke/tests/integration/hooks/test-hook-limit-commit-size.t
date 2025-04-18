# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"

  $ BYTE_LIMIT=10
  $ hook_test_setup \
  > limit_commit_size <(
  >   cat <<CONF
  > bypass_commit_string="@allow-large-files"
  > config_json='''{
  >   "commit_size_limit": $BYTE_LIMIT,
  >   "too_many_files_message": "Too many files: \${count} > \${limit}.",
  >   "too_large_message": "Too large: \${size} > \${limit}."
  > }'''
  > CONF
  > )

Small commit
  $ hg up -q "min(all())"
  $ for x in $(seq $BYTE_LIMIT); do echo -n 1 > $x; done
  $ hg ci -Aqm 1
  $ hg push -r . --to master_bookmark
  pushing rev e6f2d01a954a to destination mono:repo bookmark master_bookmark
  searching for changes
  adding changesets
  adding manifests
  adding file changes
  updating bookmark master_bookmark

Large file
  $ LARGE_CONTENT=$(for _ in $(seq $(( $BYTE_LIMIT + 1 ))); do echo -n 1; done)
  $ hg up -q "min(all())"
  $ echo -n "$LARGE_CONTENT" > largefile
  $ hg ci -Aqm largefile
  $ hg push -r . --to master_bookmark
  pushing rev b4b4dcaa16f9 to destination mono:repo bookmark master_bookmark
  searching for changes
  remote: Command failed
  remote:   Error:
  remote:     hooks failed:
  remote:     limit_commit_size for b4b4dcaa16f97662c6a6e70b6eb8c3af1aea8253: Too large: 11 > 10.
  abort: unexpected EOL, expected netstring digit
  [255]

Large commit
  $ hg up -q "min(all())"
  $ for x in $(seq $(( $BYTE_LIMIT + 1))); do echo -n 1 > "${x}_b"; done
  $ hg ci -Aqm largecommit
  $ hg push -r . --to master_bookmark
  pushing rev 0d437325fdc4 to destination mono:repo bookmark master_bookmark
  searching for changes
  remote: Command failed
  remote:   Error:
  remote:     hooks failed:
  remote:     limit_commit_size for 0d437325fdc4006bbd174b823446331bfa53a68d: Too large: 11 > 10.
  abort: unexpected EOL, expected netstring digit
  [255]

Bypass
  $ hg commit --amend -m "@allow-large-files"
  $ hg push -r . --to master_bookmark
  pushing rev dcf66a8e39a7 to destination mono:repo bookmark master_bookmark
  searching for changes
  adding changesets
  adding manifests
  adding file changes
  updating bookmark master_bookmark

Removing files whose total size is large should work
  $ hg up master_bookmark
  12 files updated, 0 files merged, 0 files removed, 0 files unresolved
  $ for x in $(seq $(( $BYTE_LIMIT + 1))); do rm "${x}_b"; done
  $ hg ci -Aqm largeremove
  $ hg status --rev ".^::."
  R 10_b
  R 11_b
  R 1_b
  R 2_b
  R 3_b
  R 4_b
  R 5_b
  R 6_b
  R 7_b
  R 8_b
  R 9_b
  $ hg push -r . --to master_bookmark
  pushing rev f4021c22aa2d to destination mono:repo bookmark master_bookmark
  searching for changes
  adding changesets
  adding manifests
  adding file changes
  updating bookmark master_bookmark
