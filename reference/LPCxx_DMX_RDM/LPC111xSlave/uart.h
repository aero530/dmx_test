/*******************************************************************************
*   uart.h:  Header file for uart driver
*
*   Copyright(C) 2011, NXP Semiconductor
*   All rights reserved.
*
*   History
*   2011.06.03  ver 1.01    Special implementation for DMX bus
*
*******************************************************************************/

#ifndef __UART_H
#define __UART_H

#define DMX_MAX_SLOTS   513

void     Uart_Init(void);
void     Uart_LoopBackTest(bool enable);
void     Uart_XmitDmxSlotValues(uint8_t *pBuf, uint16_t nrOfSlots,
                                uint16_t break_time, uint16_t mab_time,
                                bool blocking_write);
bool     Uart_XmitDmxDone(void);
uint16_t Uart_RecvDmxSlotValues(uint8_t **pbuf, bool blocking_read, uint32_t wait_msec);

#endif /* end __UART_H */

/* End Of File */
