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

#cuckoo_miner (lean_cpu) sources
set (CUCKOO_LEAN_MINER_SOURCES cuckoo/src/crypto/siphash.h cuckoo/src/cuckoo/cuckoo.h cuckoo/src/cuckoo/lean.hpp cuckoo/src/cuckoo/lean.cpp ${BLAKE_2B})

#cuckatoo_miner (lean_cpu) sources
set (CUCKATOO_LEAN_MINER_SOURCES cuckoo/src/crypto/siphash.h cuckoo/src/cuckatoo/cuckatoo.h cuckoo/src/cuckatoo/lean.hpp cuckoo/src/cuckatoo/lean.cpp ${BLAKE_2B})

#cuckoo mean miner sources (mean_cpu)
set (CUCKOO_MEAN_MINER_SOURCES cuckoo/src/crypto/siphash.h cuckoo/src/cuckoo/cuckoo.h cuckoo/src/cuckoo/mean.hpp cuckoo/src/cuckoo/mean.cpp ${BLAKE_2B})

#cuckatoo mean miner sources (mean_cpu)
set (CUCKATOO_MEAN_MINER_SOURCES cuckoo/src/crypto/siphash.h cuckoo/src/cuckatoo/cuckatoo.h cuckoo/src/cuckatoo/mean.hpp cuckoo/src/cuckatoo/mean.cpp ${BLAKE_2B})

#cuckoo cuda miner source (mean_miner.cu)
set (CUCKOO_CUDA_MINER_SOURCES cuckoo/src/cuckoo/mean.cu ${BLAKE_2B} )

#cuckoo cuda miner source (mean_miner.cu)
set (CUCKATOO_CUDA_MINER_SOURCES cuckoo/src/cuckatoo/mean.cu ${BLAKE_2B} )

###cuckoo lean_cpu miner targets
foreach(CUCKOO_SIZE 16 30 31)
    add_library(cuckoo_lean_cpu_${CUCKOO_SIZE} SHARED ${CUCKOO_LEAN_MINER_SOURCES})
    #get edgebits
    math (EXPR EDGEBITS ${CUCKOO_SIZE}-1)
    set_target_properties(cuckoo_lean_cpu_${CUCKOO_SIZE} PROPERTIES COMPILE_FLAGS "${GPP_FLAGS} -DATOMIC -DEDGEBITS=${EDGEBITS}" PREFIX "" SUFFIX ".cuckooplugin" )
endforeach(CUCKOO_SIZE)

###cuckatoo lean_cpu miner targets
foreach(CUCKATOO_SIZE 19 29 30)
    add_library(cuckatoo_lean_cpu_${CUCKATOO_SIZE} SHARED ${CUCKATOO_LEAN_MINER_SOURCES})
    #get edgebits
    math (EXPR EDGEBITS ${CUCKATOO_SIZE})
    set_target_properties(cuckatoo_lean_cpu_${CUCKATOO_SIZE} PROPERTIES COMPILE_FLAGS "${GPP_FLAGS} -DATOMIC -DEDGEBITS=${EDGEBITS}" PREFIX "" SUFFIX ".cuckooplugin" )
endforeach(CUCKATOO_SIZE)

#cuckoo mean (mean_cpu) targets
foreach(CUCKOO_SIZE 16 30 31)
    if (CUCKOO_SIZE EQUAL 16)
       set (DXBITSFLAG "-DXBITS=0")
    else()
       set (DXBITSFLAG "")
    endif()

    add_library(cuckoo_mean_cpu_${CUCKOO_SIZE} SHARED ${CUCKOO_MEAN_MINER_SOURCES})
    #get edgebits
    math (EXPR EDGEBITS ${CUCKOO_SIZE}-1)
    set_target_properties(cuckoo_mean_cpu_${CUCKOO_SIZE} PROPERTIES COMPILE_FLAGS "${GPP_FLAGS} -mavx2 ${DXBITSFLAG} -DNSIPHASH=8 -DSAVEEDGES -DEDGEBITS=${EDGEBITS}" PREFIX "" SUFFIX ".cuckooplugin")
endforeach(CUCKOO_SIZE)

#matrix miner (mean_cpu) compatible mode targets, for older processors
foreach(CUCKOO_SIZE 16 30 31)
    if (CUCKOO_SIZE EQUAL 16)
       set (DXBITSFLAG "-DXBITS=0")
    else()
       set (DXBITSFLAG "")
    endif()
    add_library(cuckoo_mean_compat_cpu_${CUCKOO_SIZE} SHARED ${CUCKOO_MEAN_MINER_SOURCES})
    #get edgebits
    math (EXPR EDGEBITS ${CUCKOO_SIZE}-1)
    set_target_properties(cuckoo_mean_compat_cpu_${CUCKOO_SIZE} PROPERTIES COMPILE_FLAGS "${GPP_FLAGS} ${DXBITSFLAG} -DNSIPHASH=1 -DSAVEEDGES -DEDGEBITS=${EDGEBITS}" PREFIX "" SUFFIX ".cuckooplugin")
endforeach(CUCKOO_SIZE)

#cuckatoo mean (cuckatoo_mean_cpu) targets
foreach(CUCKATOO_SIZE 19 29 30)
    if (CUCKATOO_SIZE EQUAL 19)
       set (DXBITSFLAG "-DXBITS=2")
    else()
       set (DXBITSFLAG "")
    endif()

    add_library(cuckatoo_mean_cpu_${CUCKATOO_SIZE} SHARED ${CUCKATOO_MEAN_MINER_SOURCES})
    #get edgebits
    math (EXPR EDGEBITS ${CUCKATOO_SIZE})
    set_target_properties(cuckatoo_mean_cpu_${CUCKATOO_SIZE} PROPERTIES COMPILE_FLAGS "${GPP_FLAGS} -mavx2 ${DXBITSFLAG} -DNSIPHASH=8 -DSAVEEDGES -DEDGEBITS=${EDGEBITS}" PREFIX "" SUFFIX ".cuckooplugin")
endforeach(CUCKATOO_SIZE)

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
    set_target_properties(cuckatoo_mean_compat_cpu_${CUCKATOO_SIZE} PROPERTIES COMPILE_FLAGS "${GPP_FLAGS} ${DXBITSFLAG} -DNSIPHASH=1 -DSAVEEDGES -DEDGEBITS=${EDGEBITS}" PREFIX "" SUFFIX ".cuckooplugin")
endforeach(CUCKATOO_SIZE)

# Cuda
if(BUILD_CUDA_PLUGINS)
    include("cmake/find_cuda.cmake")
    IF (CUDA_FOUND)
      set (CUDA_PROPAGATE_HOST_FLAGS ON)
        ##cuckoo cuda miner targets
        foreach(CUCKOO_SIZE 30 31)
            #get edgebits
            math (EXPR EDGEBITS ${CUCKOO_SIZE}-1)
        cuda_add_library (cuckoo_cuda_${CUCKOO_SIZE} SHARED ${CUCKOO_CUDA_MINER_SOURCES} OPTIONS "-DEDGEBITS=${EDGEBITS}")
        set_target_properties(cuckoo_cuda_${CUCKOO_SIZE} PROPERTIES PREFIX "" SUFFIX ".cuckooplugin")
        endforeach(CUCKOO_SIZE)
        ##cuckatoo cuda miner targets
        foreach(CUCKATOO_SIZE 19 29 30)
            #get edgebits
            math (EXPR EDGEBITS ${CUCKATOO_SIZE})
        cuda_add_library (cuckatoo_cuda_${CUCKATOO_SIZE} SHARED ${CUCKATOO_CUDA_MINER_SOURCES} OPTIONS "-DEDGEBITS=${EDGEBITS}")
        set_target_properties(cuckatoo_cuda_${CUCKATOO_SIZE} PROPERTIES PREFIX "" SUFFIX ".cuckooplugin")
        endforeach(CUCKATOO_SIZE)
    endif(CUDA_FOUND)
endif(BUILD_CUDA_PLUGINS)
