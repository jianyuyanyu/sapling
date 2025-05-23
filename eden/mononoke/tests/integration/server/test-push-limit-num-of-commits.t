# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"

setup configuration
  $ UNBUNDLE_COMMIT_LIMIT=2 setup_common_config

  $ cd $TESTTMP

setup repo

  $ hginit_treemanifest repo
  $ cd repo
  $ echo "a file content" > a
  $ hg add a
  $ hg ci -ma

setup master bookmarks

  $ hg bookmark master_bookmark -r 'tip'

verify content
  $ hg log
  commit:      0e7ec5675652
  bookmark:    master_bookmark
  user:        test
  date:        Thu Jan 01 00:00:00 1970 +0000
  summary:     a
   (re)

  $ cd $TESTTMP
  $ blobimport repo/.hg repo

setup push source repo
  $ hg clone -q mono:repo repo2

start mononoke

  $ start_and_wait_for_mononoke_server
create new commit in repo2 and check that push fails

  $ cd repo2
  $ echo "1" >> a
  $ hg addremove
  $ hg ci -ma

  $ hg push -r . --to master_bookmark
  pushing rev 2b761f0782ab to destination mono:repo bookmark master_bookmark
  searching for changes
  updating bookmark master_bookmark


  $ echo "1" >> a
  $ hg ci -maa
  $ echo "1" >> a
  $ hg ci -maaa
  $ echo "1" >> a
  $ hg ci -maaaa
  $ hg push -r . --to master_bookmark
  pushing rev 3a090ff5a2b7 to destination mono:repo bookmark master_bookmark
  searching for changes
  remote: Command failed
  remote:   Error:
  remote:     bundle2_resolver error
  remote: 
  remote:     Caused by:
  remote:         0: While resolving Changegroup
  remote:         1: Trying to push too many commits! Limit is 2, tried to push 3
  abort: unexpected EOL, expected netstring digit
  [255]
