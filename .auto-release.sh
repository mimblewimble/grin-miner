#!/bin/bash

repo_slug="mimblewimble/grin-miner"
token="$GITHUB_TOKEN"

tagname=`git describe --tags --exact-match 2>/dev/null || git symbolic-ref -q --short HEAD`
deploy_dir="deploy/grin-miner-$tagname"

echo 'make a tarball for the release binary...\n'

if [[ $TRAVIS_OS_NAME == 'osx' ]]; then

    # Do some custom requirements on OS X
    mkdir -p $deploy_dir; cp grin-miner.toml $deploy_dir; cp -R target/release/plugins $deploy_dir
    cp target/release/grin-miner $deploy_dir
    cd deploy ; rm -f *.tgz; tar zcf "grin-miner-$tagname-$TRAVIS_JOB_ID-osx.tgz" *
    /bin/ls -ls *.tgz  | awk '{print $6,$7,$8,$9,$10}'
    md5 "grin-miner-$tagname-$TRAVIS_JOB_ID-osx.tgz" > "grin-miner-$tagname-$TRAVIS_JOB_ID-osx.tgz"-md5sum.txt
    /bin/ls -ls *-md5sum.txt  | awk '{print $6,$7,$8,$9,$10}'
    cd - > /dev/null;
    echo "tarball generated\n"

    # Only generate changelog on Linux platform, to avoid duplication
    exit 0
else
    # Do some custom requirements on Linux
    mkdir -p $deploy_dir; cp grin-miner.toml $deploy_dir; cp -R target/release/plugins $deploy_dir
    cp target/release/deps/libocl_cuckatoo.so "$deploy_dir/plugins/ocl_cuckatoo.cuckooplugin"
    cp target/release/grin-miner $deploy_dir
    cd deploy ; rm -f *.tgz; tar zcf "grin-miner-$tagname-$TRAVIS_JOB_ID-linux-amd64.tgz" *
    /bin/ls -ls *.tgz  | awk '{print $6,$7,$8,$9,$10}'
    md5sum "grin-miner-$tagname-$TRAVIS_JOB_ID-linux-amd64.tgz" > "grin-miner-$tagname-$TRAVIS_JOB_ID-linux-amd64.tgz"-md5sum.txt
    /bin/ls -ls *-md5sum.txt  | awk '{print $6,$7,$8,$9,$10}'
    cd - > /dev/null;
    echo "tarball generated\n"
fi

version="$tagname"
branch="`git symbolic-ref -q --short HEAD`"

# automatic changelog generator
gem install github_changelog_generator

LAST_REVISION=$(git rev-list --tags --skip=1 --max-count=1)
LAST_RELEASE_TAG=$(git describe --abbrev=0 --tags ${LAST_REVISION})

# Generate CHANGELOG.md
github_changelog_generator \
  -u $(cut -d "/" -f1 <<< $repo_slug) \
  -p $(cut -d "/" -f2 <<< $repo_slug) \
  --token $token \
  --since-tag ${LAST_RELEASE_TAG}

body="$(cat CHANGELOG.md)"

# Overwrite CHANGELOG.md with JSON data for GitHub API
jq -n \
  --arg body "$body" \
  --arg name "$version" \
  --arg tag_name "$version" \
  --arg target_commitish "$branch" \
  '{
    body: $body,
    name: $name,
    tag_name: $tag_name,
    target_commitish: $target_commitish,
    draft: false,
    prerelease: false
  }' > CHANGELOG.md

echo "Create release $version for repo: $repo_slug, branch: $branch"
curl -H "Authorization: token $token" --data @CHANGELOG.md "https://api.github.com/repos/$repo_slug/releases"
echo "auto changelog uploaded.\n"

