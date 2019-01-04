#!/bin/bash

if [[ $TRAVIS_OS_NAME == 'osx' ]]; then
    ;;
else
    # linux 
    wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu1604/x86_64/cuda-repo-ubuntu1604_10.0.130-1_amd64.deb
    sudo dpkg -i cuda-repo-ubuntu1604_10.0.130-1_amd64.deb
    sudo apt-key adv --fetch-keys http://developer.download.nvidia.com/compute/cuda/repos/ubuntu1604/x86_64/7fa2af80.pub
    sudo apt-get update
    sudo apt-get install cuda
fi