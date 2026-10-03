#include "EthTrcv.h"

/*
 * Demonstrates calling the Rust-implemented AUTOSAR
 * EthTrcv driver from C.
 *
 * Links against the Rust binary which exports
 * EthTrcv_Init and EthTrcv_SetTransceiverMode
 * with C ABI.
 */
void c_demo(void)
{
    Std_ReturnType ret;

    EthTrcv_Init(0);

    ret = EthTrcv_SetTransceiverMode(
        0,
        ETHTRCV_MODE_ACTIVE
    );

    if (ret != E_OK) {
        /* handle error */
        return;
    }
}
