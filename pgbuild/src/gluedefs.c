#include <stdint.h>
#include <postgres.h>
#include <access/skey.h>
#include <access/relscan.h>
#include <utils/syscache.h>
#include <utils/rel.h>
#include <setjmp.h>
#include <stddef.h>

uint32_t pg_version() { return PG_VERSION_NUM / 100; }
uint32_t func_max_args() { return FUNC_MAX_ARGS; }
uint32_t index_max_keys() { return INDEX_MAX_KEYS; }
uint32_t namedatalen() { return NAMEDATALEN; }
uint32_t float4_byval() { return FLOAT4PASSBYVAL; }
uint32_t float8_byval() { return FLOAT8PASSBYVAL; }
uint32_t len_scankeydata() { return sizeof(ScanKeyData); }
uint32_t len_sigjmpbuf() { return sizeof(sigjmp_buf); }

uint32_t cacheid_typeoid() { return TYPEOID; }

uint32_t relatt_offset() { return offsetof(RelationData, rd_att); }
uint32_t rs_cbuf_offset() { return offsetof(HeapScanDescData, rs_cbuf); }
uint32_t xs_cbuf_offset() { return offsetof(IndexScanDescData, xs_cbuf); }
