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

## CUCKATOO

#flags applying to all cuckatoo variants
set (CUCKATOO_BUILD_FLAGS "-DC_CALL_CONVENTION=1 -DSQUASH_OUTPUT=1")

#cuckatoo lean (lean_cpu) sources
set (CUCKATOO_LEAN_MINER_SOURCES cuckoo/src/crypto/siphash.h cuckoo/src/cuckatoo/cuckatoo.h cuckoo/src/cuckatoo/lean.hpp cuckoo/src/cuckatoo/lean.cpp ${BLAKE_2B})

#cuckatoo mean miner sources (mean_cpu)
set (CUCKATOO_MEAN_MINER_SOURCES cuckoo/src/crypto/siphash.h cuckoo/src/cuckatoo/cuckatoo.h cuckoo/src/cuckatoo/mean.hpp cuckoo/src/cuckatoo/mean.cpp ${BLAKE_2B})

#cuckatoo cuda mean miner source (mean_miner.cu)
set (CUCKATOO_CUDA_MEAN_MINER_SOURCES cuckoo/src/cuckatoo/mean.cu ${BLAKE_2B} )

#cuckatoo cuda LEAN miner source (mean.cu)
set (CUCKATOO_CUDA_LEAN_MINER_SOURCES cuckoo/src/cuckatoo/lean.cu ${BLAKE_2B} )

### CUCKAROO

#flags applying to all cuckatoo variants
set (CUCKAROO_BUILD_FLAGS "-DC_CALL_CONVENTION=1 -DSQUASH_OUTPUT=1")

#cuckaroo cuda mean miner source (mean.cu)
set (CUCKAROO_CUDA_MEAN_MINER_SOURCES cuckoo/src/cuckaroo/mean.cu ${BLAKE_2B} )

###cuckatoo lean_cpu miner targets
foreach(CUCKATOO_SIZE 19 29 30)
    add_library(cuckatoo_lean_cpu_${CUCKATOO_SIZE} SHARED ${CUCKATOO_LEAN_MINER_SOURCES})
    #get edgebits
    math (EXPR EDGEBITS ${CUCKATOO_SIZE})
    set_target_properties(cuckatoo_lean_cpu_${CUCKATOO_SIZE} PROPERTIES COMPILE_FLAGS "${GPP_FLAGS} ${CUCKATOO_BUILD_FLAGS} -DATOMIC -DEDGEBITS=${EDGEBITS}" PREFIX "" SUFFIX ".cuckooplugin" )
endforeach(CUCKATOO_SIZE)

set (CUCKATOO_MEAN_COMPAT_FLAGS "${CUCKATOO_BUILD_FLAGS} -mno-avx2 -DNSIPHASH=4 -DSAVEEDGES")

#cuckatoo_mean_compat_cpu compatible mode targets, for older processors
foreach(CUCKATOO_SIZE 19 29 30 31)
    set (DXBITSFLAG "")
    set (DEXPANDFLAGS "")
    if (CUCKATOO_SIZE EQUAL 19)
       set (DXBITSFLAG "-DXBITS=2")
    endif()
    if (CUCKATOO_SIZE EQUAL 30)
       set (DEXPANDFLAGS "-DEXPANDROUND=10 -DCOMPRESSROUND=22")
    endif()
    if (CUCKATOO_SIZE EQUAL 31)
       set (DEXPANDFLAGS "-DEXPANDROUND=8 -DCOMPRESSROUND=22")
       set (DXBITSFLAG "-DXBITS=8")
    endif()
    add_library(cuckatoo_mean_compat_cpu_${CUCKATOO_SIZE} SHARED ${CUCKATOO_MEAN_MINER_SOURCES})
    #get edgebits
    math (EXPR EDGEBITS ${CUCKATOO_SIZE})
    set_target_properties(cuckatoo_mean_compat_cpu_${CUCKATOO_SIZE} PROPERTIES COMPILE_FLAGS "${GPP_FLAGS} ${DXBITSFLAG} ${CUCKATOO_MEAN_COMPAT_FLAGS} ${DEXPANDFLAGS} -DEDGEBITS=${EDGEBITS}" PREFIX "" SUFFIX ".cuckooplugin")
endforeach(CUCKATOO_SIZE)

set (CUCKATOO_MEAN_AVX2_FLAGS "${CUCKATOO_BUILD_FLAGS} -mavx2 -DNSIPHASH=8 -DSAVEEDGES")

#cuckatoo mean avx2 (cuckatoo_mean_avx2_cpu) targets
foreach(CUCKATOO_SIZE 19 29 30 31)
    set (DXBITSFLAG "")
    set (DEXPANDFLAGS "")
    if (CUCKATOO_SIZE EQUAL 19)
       set (DXBITSFLAG "-DXBITS=2")
    endif()
    if (CUCKATOO_SIZE EQUAL 30)
       set (DEXPANDFLAGS "-DEXPANDROUND=10 -DCOMPRESSROUND=22")
    endif()
    if (CUCKATOO_SIZE EQUAL 31)
       set (DEXPANDFLAGS "-DEXPANDROUND=8 -DCOMPRESSROUND=22")
       set (DXBITSFLAG "-DXBITS=8")
    endif()

    add_library(cuckatoo_mean_avx2_cpu_${CUCKATOO_SIZE} SHARED ${CUCKATOO_MEAN_MINER_SOURCES})
    #get edgebits
    math (EXPR EDGEBITS ${CUCKATOO_SIZE})
    set_target_properties(cuckatoo_mean_avx2_cpu_${CUCKATOO_SIZE} PROPERTIES COMPILE_FLAGS "${GPP_FLAGS} ${DXBITSFLAG} ${CUCKATOO_MEAN_AVX2_FLAGS} ${DEXPANDFLAGS} -DEDGEBITS=${EDGEBITS}" PREFIX "" SUFFIX ".cuckooplugin")
endforeach(CUCKATOO_SIZE)

# Cuda
if(BUILD_CUDA_PLUGINS)
    include("cmake/find_cuda.cmake")
    IF (CUDA_FOUND)
      set (CUDA_PROPAGATE_HOST_FLAGS ON)
        ##cuckatoo mean cuda mean miner targets
        foreach(CUCKATOO_SIZE 29 30)
            #get edgebits
            math (EXPR EDGEBITS ${CUCKATOO_SIZE})
        cuda_add_library (cuckatoo_mean_cuda_${CUCKATOO_SIZE} SHARED ${CUCKATOO_CUDA_MEAN_MINER_SOURCES} OPTIONS "-DEDGEBITS=${EDGEBITS} ${CUCKATOO_BUILD_FLAGS}")
        set_target_properties(cuckatoo_mean_cuda_${CUCKATOO_SIZE} PROPERTIES PREFIX "" SUFFIX ".cuckooplugin")
        endforeach(CUCKATOO_SIZE)
        ##cuckatoo mean cuda mean miner targets
        foreach(CUCKATOO_SIZE 31)
            #get edgebits
            math (EXPR EDGEBITS ${CUCKATOO_SIZE})
        cuda_add_library (cuckatoo_mean_cuda_${CUCKATOO_SIZE} SHARED ${CUCKATOO_CUDA_MEAN_MINER_SOURCES} OPTIONS "-DXBITS=7 -DEDGEBITS=${EDGEBITS} ${CUCKATOO_BUILD_FLAGS}")
        set_target_properties(cuckatoo_mean_cuda_${CUCKATOO_SIZE} PROPERTIES PREFIX "" SUFFIX ".cuckooplugin")
        endforeach(CUCKATOO_SIZE)
        ##cuckatoo lean cuda mean miner targets
        foreach(CUCKATOO_SIZE 29 30 31)
            #get edgebits
            math (EXPR EDGEBITS ${CUCKATOO_SIZE})
        cuda_add_library (cuckatoo_lean_cuda_${CUCKATOO_SIZE} SHARED ${CUCKATOO_CUDA_LEAN_MINER_SOURCES} OPTIONS "-DEDGEBITS=${EDGEBITS} ${CUCKATOO_BUILD_FLAGS}")
        set_target_properties(cuckatoo_lean_cuda_${CUCKATOO_SIZE} PROPERTIES PREFIX "" SUFFIX ".cuckooplugin")
        endforeach(CUCKATOO_SIZE)

        ##cuckaroo mean cuda mean miner targets
        foreach(CUCKAROO_SIZE 19 29)
            #get edgebits
            math (EXPR EDGEBITS ${CUCKAROO_SIZE})
        cuda_add_library (cuckaroo_mean_cuda_${CUCKAROO_SIZE} SHARED ${CUCKAROO_CUDA_MEAN_MINER_SOURCES} OPTIONS "-DEDGEBITS=${EDGEBITS} ${CUCKAROO_BUILD_FLAGS}")
        set_target_properties(cuckaroo_mean_cuda_${CUCKAROO_SIZE} PROPERTIES PREFIX "" SUFFIX ".cuckooplugin")
        endforeach(CUCKAROO_SIZE)
    endif(CUDA_FOUND)
endif(BUILD_CUDA_PLUGINS)
