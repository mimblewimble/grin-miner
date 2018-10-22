cmake_minimum_required(VERSION 3.2)
project (CuckooMinerPlugins)

set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/plugins)
set (CMAKE_CXX_FLAGS "--std=c++11")

set (OPT "-O3")
set (DOPT "-DPREFETCH")

set (FLAGS "-Wno-format -Wno-deprecated-declarations -D_POSIX_C_SOURCE=200112L ${OPT} ${DOPT} -I. ${CPPFLAGS} -pthread")
set (GPP_FLAGS "-march=native -m64 ${FLAGS}")
set (CFLAGS "-Wno-format -fomit-frame-pointer ${OPT}")
set (GCC_FLAGS "-m64 -std=gnu11 ${CFLAGS}")

set (CUDA_HOST_COMPILER_OVERRIDE $ENV{CUDA_HOST_COMPILER})

#blake2b prerequisite
set (BLAKE_2B "cuckoo/src/crypto/blake2b-ref.c")

#flags applying to all cuckatoo variants
set (CUCKATOO_BUILD_FLAGS "-DC_CALL_CONVENTION=1 -DSQUASH_OUTPUT=1")

#cuckatoo mean miner sources (mean_cpu)
set (CUCKATOO_MEAN_MINER_SOURCES cuckoo/src/crypto/siphash.h cuckoo/src/cuckatoo/cuckatoo.h cuckoo/src/cuckatoo/mean.hpp cuckoo/src/cuckatoo/mean.cpp ${BLAKE_2B})

set (CUCKATOO_MEAN_COMPAT_FLAGS "${CUCKATOO_BUILD_FLAGS} -DNSIPHASH=1 -DSAVEEDGES")

#matrix miner (cuckatoo_mean_compat_cpu) compatible mode targets, for older processors
foreach(CUCKATOO_SIZE 19 29 30)
    if (CUCKATOO_SIZE EQUAL 19)
       set (DXBITSFLAG "-DXBITS=2")
    else()
       set (DXBITSFLAG "")
    endif()
    add_library(cuckatoo_mean_compat_cpu_${CUCKATOO_SIZE} SHARED ${CUCKATOO_MEAN_MINER_SOURCES})
    #get edgebits
    math (EXPR EDGEBITS ${CUCKATOO_SIZE})
    set_target_properties(cuckatoo_mean_compat_cpu_${CUCKATOO_SIZE} PROPERTIES COMPILE_FLAGS "${GPP_FLAGS} ${DXBITSFLAG} ${CUCKATOO_MEAN_COMPAT_FLAGS} -DEDGEBITS=${EDGEBITS}" PREFIX "" SUFFIX ".cuckooplugin")
endforeach(CUCKATOO_SIZE)

