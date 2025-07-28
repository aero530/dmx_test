/*******************************************************************************
*   uart.c:  UART API file for NXP LPC11xx
*
*   Copyright(C) 2011, NXP Semiconductor
*   All rights reserved.
*
*   History
*   2011.06.03  ver 1.01    Special implementation for DMX bus
*
*******************************************************************************/

#include <stdbool.h>

#include "bsp.h"
#include "uart.h"

//#define ERROR_COUNTERS

#if   (LPC_CORE_CLOCKSPEED_HZ == 12000000) || (LPC_CORE_CLOCKSPEED_HZ == 36000000)
  #define UART_PCLK   12000000
#elif (LPC_CORE_CLOCKSPEED_HZ == 24000000) || (LPC_CORE_CLOCKSPEED_HZ == 48000000)
  #define UART_PCLK   24000000
#else
  #error "This Core clock speed isn't supported by this SW yet!"
#endif

// bit definitions of uart IER register
#define IER_RBR     0x01
#define IER_THRE    0x02
#define IER_RXL     0x04
#define IER_ABEO    0x10
#define IER_ABTO    0x20

// value definitions for uart IIR register
#define IIR_RLS     0x06
#define IIR_RDA     0x04
#define IIR_CTI     0x0C
#define IIR_THRE    0x02
#define IIR_MODEM   0x01

// bit definitions of uart LSR register
#define LSR_RDR     0x01
#define LSR_OE      0x02
#define LSR_PE      0x04
#define LSR_FE      0x08
#define LSR_BI      0x10
#define LSR_THRE    0x20
#define LSR_TEMT    0x40
#define LSR_RXFE    0x80

#define SC_RDM              0xCC      /* Start Code RDM Packet */
#define SC_SUB_MESSAGE      0x01
#define CRC_LEN             2         /* 2 byte (16 bit) */
#define PREAMBLE_SEPARATOR  0xAA

static uint8_t       slot_values_1[DMX_MAX_SLOTS]; /* slot[0] is dmx start code */
static uint8_t       slot_values_2[DMX_MAX_SLOTS]; /* slot[0] is dmx start code */
static uint8_t     * pSlotValues;

static bool          dmx_recv_data_copied;
static bool          dmx_recv_error;
static uint16_t      dmx_recv_slot_nr;

static volatile uint8_t * dmx_recv_buf;
static volatile uint16_t  dmx_recv_buf_len;

static volatile bool dmx_xmit_data_done;
static uint8_t     * dmx_xmit_data;
static uint16_t      dmx_xmit_data_len;

#ifdef SUPPORT_LOOPBACK_TEST
static bool          dmx_loopback_test;
#endif
static uint8_t       dmx_counter_id;

#ifdef ERROR_COUNTERS
static uint32_t      errCntOE;
static uint32_t      errCntPE;
static uint32_t      errCntFE;
static uint32_t      errCntRX;
#endif

/*******************************************************************************
* Local functions
*******************************************************************************/

static void PassDMXdataToApp(void)
{
    if (!dmx_recv_data_copied &&
        !dmx_recv_error &&
        (dmx_recv_slot_nr > 0) &&
        (dmx_recv_buf == 0))
    {
        dmx_recv_data_copied = true;
        dmx_recv_buf_len = dmx_recv_slot_nr;
        dmx_recv_buf = pSlotValues;
        if (pSlotValues == slot_values_1)
        {
            pSlotValues = slot_values_2;
        }
        else
        {
            pSlotValues = slot_values_1;
        }
        dmx_recv_slot_nr = DMX_MAX_SLOTS;  // block receive data till BREAK
    }
}

static void StoreSlotValue(uint8_t dmx_data)
{
    if (dmx_recv_slot_nr < DMX_MAX_SLOTS)
    {
        if (dmx_recv_slot_nr == 0)
        {   // clear RDM Packet length field
            pSlotValues[2] = 0;
        }
        pSlotValues[dmx_recv_slot_nr++] = dmx_data;
        if (dmx_recv_slot_nr == DMX_MAX_SLOTS)
        {
            PassDMXdataToApp();
        }
        else if ((pSlotValues[0] == SC_RDM) &&
                 (pSlotValues[1] == SC_SUB_MESSAGE) &&
                 (pSlotValues[2] != 0) &&
                 (dmx_recv_slot_nr == pSlotValues[2] + CRC_LEN))
        {
            PassDMXdataToApp();
        }
    }
}

static void Uart_XmitEmpty(void)
{
    while (!(LPC_UART->LSR & LSR_TEMT));
}

#ifdef SUPPORT_LOOPBACK_TEST
static void LoopBackTest(uint16_t slot1, uint16_t slot2, uint16_t slot3, uint16_t slot4)
{
static uint8_t testDataBuf[DMX_MAX_SLOTS] = {0}; /* slot[0] is dmx start code */

    testDataBuf[0] = 0;    // NULL start code
    if ((slot1 > 0) && (slot1 < DMX_MAX_SLOTS)) testDataBuf[slot1] += 1;
    if ((slot2 > 0) && (slot2 < DMX_MAX_SLOTS)) testDataBuf[slot2] += 1;
    if ((slot3 > 0) && (slot3 < DMX_MAX_SLOTS)) testDataBuf[slot3] += 1;
    if ((slot4 > 0) && (slot4 < DMX_MAX_SLOTS)) testDataBuf[slot4] += 1;
    Uart_XmitDmxSlotValues(testDataBuf, DMX_MAX_SLOTS, 176, 12, true);
}
#endif

/*******************************************************************************
* Global functions
*******************************************************************************/

/*******************************************************************************
* Function    : UART_IRQHandler()
* Description :	UART interrupt handler
* Argument(s) : none.
* Return(s)   : none.
*******************************************************************************/

void UART_IRQHandler(void)
{
    uint8_t IIRValue = LPC_UART->IIR & 0x0F;
    
    if (IIRValue == IIR_RLS)              /* Receive Line Status */
    {
        uint8_t LSRValue = LPC_UART->LSR; /* Read LSR clears the interrupt */
        
        if (LSRValue & LSR_BI)            /* break interrupt */
        {
            LPC_UART->RBR;                /* remove the break from fifo */
            PassDMXdataToApp();
            dmx_recv_data_copied = false;
            dmx_recv_slot_nr = 0;
            dmx_recv_error = false;
            return;
        }
        else if (LSRValue & (LSR_OE|LSR_PE|LSR_FE|LSR_RXFE))  /* There are errors */
        {
#ifdef ERROR_COUNTERS
            if (LSRValue & LSR_OE)        /* Fifo overrun error */
            {
                errCntOE += 1;
            }
            else if (LSRValue & LSR_PE)   /* Parity error */
            {
                errCntPE += 1;
            }
            else if (LSRValue & LSR_FE)   /* Framing error */
            {
                errCntFE += 1;
            }
            else if (LSRValue & LSR_RXFE) /* There are RX errors */
            {
                errCntRX += 1;
            }
#endif
            LPC_UART->RBR;                /* remove this byte from fifo */
            dmx_recv_error = true;
            return;
        }
        if (LSRValue & LSR_RDR)	          /* Receive Data Ready */			
        {
            StoreSlotValue(LPC_UART->RBR);/* read RBR will clear the interrupt */
        }
        if (LSRValue & LSR_THRE)
        {
            if (dmx_xmit_data_len)
            {
                LPC_UART->THR = *dmx_xmit_data++;
                dmx_xmit_data_len--;
            }
            else
            {
                LPC_UART->IER &= ~IER_THRE;  /* Disable THRE interrupt */
                dmx_xmit_data_done = true;
            }
        }
    }
    else if (IIRValue & (IIR_RDA | IIR_CTI)) /* Receive Data Available or Character timeout indicator */
    {
        while(LPC_UART->LSR & 1)
        {
            StoreSlotValue(LPC_UART->RBR);/* read from FIFO */
        }
    }
    else if (IIRValue == IIR_THRE)        /* transmit holding register empty */
    {
        if (dmx_xmit_data_len)
        {
            LPC_UART->THR = *dmx_xmit_data++;
            dmx_xmit_data_len--;
        }
        else
        {
            LPC_UART->IER &= ~IER_THRE;   /* Disable THRE interrupt */
            dmx_xmit_data_done = true;
        }
    }
}

/*******************************************************************************
* Function    : Uart_Init()
* Description : Initialize a serial port for DMX communication at 250kbaud.
* Argument(s) : none.
* Return(s)   : none.
*******************************************************************************/

void Uart_Init(void)
{
    LPC_SYSCON->UARTCLKDIV = SystemCoreClock / UART_PCLK;

    /* Init global variables */
    pSlotValues = slot_values_1;
    dmx_recv_data_copied = false;
    dmx_recv_error = false;
    dmx_recv_slot_nr = 0;
    dmx_recv_buf = 0;

#ifdef SUPPORT_LOOPBACK_TEST
    dmx_loopback_test = false;
#endif
    dmx_counter_id = bsp_claim_msec_cnt();
    
#ifdef ERROR_COUNTERS
    errCntOE = 0;
    errCntPE = 0;
    errCntFE = 0;
    errCntRX = 0;
#endif

    /* Init the uart peripheral block */
    LPC_UART->LCR = 0x80; /* Enable access to Divisor Latches (DLAB=1) */
    LPC_UART->DLM = 0x00; /* Load divisor MSB */
#if   (UART_PCLK == 12000000)
    LPC_UART->DLL = 0x03; /* Load divisor LSB */
#elif (UART_PCLK == 24000000)
    LPC_UART->DLL = 0x06; /* Load divisor LSB */
#endif
    LPC_UART->FDR = 0x10; /* Load fractional divider, 250kbaud */
    LPC_UART->LCR = 0x07; /* 8 databits, 2 stopbits, no parity (DLAB=0) */
    LPC_UART->FCR = 0x07; /* Enable and reset TX and RX FIFO, trigger level 0 */

    NVIC_SetPriority(UART_IRQn, UART_IRQ_PRIORITY);
    NVIC_EnableIRQ(UART_IRQn);

    LPC_UART->IER = IER_RXL | IER_RBR; /* Enable UART RX and line status interrupt */
}

void Uart_LoopBackTest(bool enable)
{
#ifdef SUPPORT_LOOPBACK_TEST
    dmx_loopback_test = enable;
#endif
}

/*******************************************************************************
* Function    : Uart_XmitDmxSlotValues()
* Description : Transmit DMX slot values. Max 513 slots. Slot[0] is DMX start code.
* Argument(s) :
* (I/O) pBuf      - pointer to buffer which holds the DMX slot values (pbuf[0] is dmx start code)
* (I)   nrOfSlots - the numbers of DMX slot values to transmit (including the dmx start code)
* (I)   break_time - break time in usec, when 0 no break will be generated
* (I)   mab_time   - mark after break time in usec
* (I)   blocking_write - bool flag to indicate wether this function must wait till data is transmitted
* Return(s)   : none.
*******************************************************************************/

void Uart_XmitDmxSlotValues(uint8_t *pBuf, uint16_t nrOfSlots,
                            uint16_t break_time, uint16_t mab_time,
                            bool blocking_write)
{
    if (nrOfSlots > 1)
    {
        dmx_xmit_data_done = false;
        dmx_xmit_data_len = nrOfSlots - 1;
        if (break_time > 0)
        {
            LPC_UART->LCR |=  (1 << 6);  /* generate line break for 92 usec */
            bsp_delay_usec(break_time);
            LPC_UART->LCR &= ~(1 << 6);  /* remove line break for 12 usec */
        }
        if (mab_time > 0) bsp_delay_usec(mab_time);
        LPC_UART->THR = *pBuf++;         /* xmit DMX start code */
        dmx_xmit_data = pBuf;
        LPC_UART->IER |= IER_THRE;       /* Enable THRE interrupt */
        if (blocking_write)
        {
            while(!dmx_xmit_data_done);
            Uart_XmitEmpty();
        }
    }
}

/*******************************************************************************
* Function    : Uart_XmitDmxDone()
* Description : This function can be called periodically to check wether the
*               non blocking write via Uart_XmitDmxSlotValues() is done
* Argument(s) : none.
* Return(s)   : true when transmit done, false when not.
*******************************************************************************/

bool Uart_XmitDmxDone(void)
{
    if (dmx_xmit_data_done)
    {
        Uart_XmitEmpty();
        return true;
    }
    return false;
}

/*******************************************************************************
* Function    : Uart_RecvDmxSlotValues()
* Description : Get the last received DMX data. At least once a second new data should be available.
* Argument(s) :
* (I/O) pBuf          - pointer to buffer in which DMX start code and requested slot values will be returned
* (I)   blocking_read - bool flag to indicate wether this function must wait till data is received
* (I)   wait_msec     - blocking time in milli seconds
* Return(s)   : 0 when no data available, >0 when new data available
*******************************************************************************/

uint16_t Uart_RecvDmxSlotValues(uint8_t **pBuf, bool blocking_read, uint32_t wait_msec)
{
static bool release_used_buffer = false;
    
    if (release_used_buffer)
    {
        release_used_buffer = false;
        dmx_recv_buf = 0;
    }
    bsp_reset_msec_cnt(dmx_counter_id);
#ifdef SUPPORT_LOOPBACK_TEST
    if (dmx_loopback_test) LoopBackTest(509, 510, 511, 512);
#endif
    while (dmx_recv_buf == 0)
    {
        if (!blocking_read || (bsp_get_msec_cnt(dmx_counter_id) > wait_msec))
        {
            return 0;
        }
    }
    *pBuf = (uint8_t *)dmx_recv_buf;
    release_used_buffer = true;
    return dmx_recv_buf_len;
}

/* EOF */
