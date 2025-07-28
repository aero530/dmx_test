/******************************************************************************/
/*  Copyright (c) 2011 NXP B.V.  All rights are reserved.                     */
/*  Reproduction in whole or in part is prohibited without the prior          */
/*  written consent of the copyright owner.                                   */
/*                                                                            */
/*  This software and any compilation or derivative thereof is, and           */
/*  shall remain the proprietary information of NXP and is                    */
/*  highly confidential in nature. Any and all use hereof is restricted       */
/*  and is subject to the terms and conditions set forth in the               */
/*  software license agreement concluded with NXP B.V.                        */
/*                                                                            */
/*  Under no circumstances is this software or any derivative thereof         */
/*  to be combined with any Open Source Software, exposed to, or in any       */
/*  way licensed under any Open License Terms without the express prior       */
/*  written permission of the copyright owner.                                */
/*                                                                            */
/*  For the purpose of the above, the term Open Source Software means         */
/*  any software that is licensed under Open License Terms. Open              */
/*  License Terms means terms in any license that require as a                */
/*  condition of use, modification and/or distribution of a work              */
/*                                                                            */
/*  1. the making available of source code or other materials                 */
/*     preferred for modification, or                                         */
/*                                                                            */
/*  2. the granting of permission for creating derivative                     */
/*     works, or                                                              */
/*                                                                            */
/*  3. the reproduction of certain notices or license terms                   */
/*     in derivative works or accompanying documentation, or                  */
/*                                                                            */
/*  4. the granting of a royalty-free license to any party                    */
/*     under Intellectual Property Rights                                     */
/*                                                                            */
/*  regarding the work and/or any work that contains, is combined with,       */
/*  requires or otherwise is based on the work.                               */
/*                                                                            */
/*  This software is provided for ease of recompilation only.                 */
/*  Modification and reverse engineering of this software are strictly        */
/*  prohibited.                                                               */
/*                                                                            */
/******************************************************************************/

/*******************************************************************************
 * standard include files
 ******************************************************************************/

#include <stdbool.h>
#include <string.h>

/*******************************************************************************
 * project include files
 ******************************************************************************/

#include "bsp.h"
#include "uart.h"
#include "wdt.h"
#include "lcd.h"
#include "timer_pwm.h"
#include "main.h"
#ifdef SUPPORT_MANUAL_CONTROL
#include "manual_ctrl.h"
#endif

/*******************************************************************************
 * Types and defines
 ******************************************************************************/

//#define VALIDATE_PERFORMANCE_ON_MAX_DMX_BUS_LOAD

#define MAN_ID    ((32*32*('N'-'@')) + (32*('X'-'@')) + ('P'-'@'))

#define DEVICE_MODEL_ID1             0x00  /* MSB */
#define DEVICE_MODEL_ID0             0x01  /* LSB */

#define SW_VERSION_ID3               0x01  /* MSB */
#define SW_VERSION_ID2               0x00
#define SW_VERSION_ID1               0x00
#define SW_VERSION_ID0               0x03  /* LSB */
#define FIRMWARE_VERSION             "V1.0.0.3"

#define DMX512_FOOTPRINT             4     /* nr of slots this device uses */

#define SC_NULL                      0x00  /* NULL Start Code Packet */
#define SC_TEXT                      0x17  /* Start Code ASCII Text Packet */
#define SC_RDM                       0xCC  /* Start Code RDM Packet */

#define SC_SUB_MESSAGE               0x01

// RDM command classes (Slot 20)
#define DISCOVERY_COMMAND            0x10
#define DISCOVERY_COMMAND_RESPONSE   0x11
#define GET_COMMAND                  0x20
#define GET_COMMAND_RESPONSE         0x21
#define SET_COMMAND                  0x30
#define SET_COMMAND_RESPONSE         0x31

#define RESPONSE_TYPE_ACK            0x00
#define RESPONSE_TYPE_ACK_TIMER      0x01
#define RESPONSE_TYPE_NACK_REASON    0x02
#define RESPONSE_TYPE_ACK_OVERFLOW   0x03

#define DISC_UNIQUE_BRANCH           0x0001
#define DISC_MUTE                    0x0002
#define DISC_UN_MUTE                 0x0003
#define DEVICE_INFO                  0x0060
#define SOFTWARE_VERSION_LABEL       0x00C0
#define DMX_START_ADDRESS            0x00F0
#define IDENTIFY_DEVICE              0x1000

/*******************************************************************************
 * global variables
 ******************************************************************************/

static uint16_t  my_dmx_start_address;

#ifdef SUPPORT_RDM_OVER_DMX512
static uint32_t  my_unique_id;
static long long my_uid;
static uint16_t  break_time;
static uint16_t  mab_time;
static uint8_t   rdm_message_count;
static uint8_t   rdm_response_msg[64];
static uint8_t   identify_state;        // off=0, on=1
#endif

/*******************************************************************************
 * local functions
 ******************************************************************************/

static uint16_t get_my_slot_nrs(void)
{
    return ((bsp_read_dip_switches() * 32) + 29);
}

static inline void init_globals(void)
{
    my_dmx_start_address = get_my_slot_nrs();
#ifdef SUPPORT_RDM_OVER_DMX512
    my_unique_id = bsp_get_id(); // this is not garanteed to be unique !!
    my_uid = ((MAN_ID >> 8) & 0xFF);
    my_uid <<= 8;
    my_uid |= ((MAN_ID >> 0) & 0xFF);
    my_uid <<= 8;
    my_uid |= ((my_unique_id >> 24) & 0xFF);
    my_uid <<= 8;
    my_uid |= ((my_unique_id >> 16) & 0xFF);
    my_uid <<= 8;
    my_uid |= ((my_unique_id >>  8) & 0xFF);
    my_uid <<= 8;
    my_uid |= ((my_unique_id >>  0) & 0xFF);
    break_time = 176;           /* 176 usec */
    mab_time   = 12;            /*  12 usec */
    rdm_message_count = 0;
    identify_state = 0;
#endif
}

#ifdef SUPPORT_RDM_OVER_DMX512
static uint16_t calc_crc(uint8_t *data, uint8_t len)
{
    uint16_t crc = 0;
    uint16_t i;
    
    for (i=0; i < len; i++)
    {
        crc += *data++;
    }
    return crc;
}

static inline void send_disc_response(uint8_t *rdm_msg)
{
    bsp_delay_usec(100);
    rdm_response_msg[0]  = 0xFE;
    rdm_response_msg[1]  = 0xFE;
    rdm_response_msg[2]  = 0xFE;
    rdm_response_msg[3]  = 0xFE;
    rdm_response_msg[4]  = 0xFE;
    rdm_response_msg[5]  = 0xFE;
    rdm_response_msg[6]  = 0xFE;
    rdm_response_msg[7]  = 0xAA;
    rdm_response_msg[8]  = ((MAN_ID >> 8) & 0xFF) | 0xAA;
    rdm_response_msg[9]  = ((MAN_ID >> 8) & 0xFF) | 0x55;
    rdm_response_msg[10] = ((MAN_ID >> 0) & 0xFF) | 0xAA;
    rdm_response_msg[11] = ((MAN_ID >> 0) & 0xFF) | 0x55;
    rdm_response_msg[12] = ((my_unique_id >> 24) & 0xFF) | 0xAA;
    rdm_response_msg[13] = ((my_unique_id >> 24) & 0xFF) | 0x55;
    rdm_response_msg[14] = ((my_unique_id >> 16) & 0xFF) | 0xAA;
    rdm_response_msg[15] = ((my_unique_id >> 16) & 0xFF) | 0x55;
    rdm_response_msg[16] = ((my_unique_id >>  8) & 0xFF) | 0xAA;
    rdm_response_msg[17] = ((my_unique_id >>  8) & 0xFF) | 0x55;
    rdm_response_msg[18] = ((my_unique_id >>  0) & 0xFF) | 0xAA;
    rdm_response_msg[19] = ((my_unique_id >>  0) & 0xFF) | 0x55;
    uint16_t crc = calc_crc(&rdm_response_msg[8], 12);
    rdm_response_msg[20] = ((crc >> 8) & 0xFF) | 0xAA;
    rdm_response_msg[21] = ((crc >> 8) & 0xFF) | 0x55;
    rdm_response_msg[22] = ((crc >> 0) & 0xFF) | 0xAA;
    rdm_response_msg[23] = ((crc >> 0) & 0xFF) | 0x55;
    bsp_xmit_to_dmx_bus();
    Uart_XmitDmxSlotValues(rdm_response_msg, 24, 0/*NO BREAK*/, 4, true);
    bsp_recv_from_dmx_bus();
}

static void send_rdm_msg(uint8_t *rdm_msg, uint8_t command_class, uint8_t data_len)
{
    uint8_t count = 24 + data_len;
    
    // Remark: Parameter Data must be filled by the caller of this function
    bsp_delay_usec(150);
    rdm_response_msg[0]  = SC_RDM;
    rdm_response_msg[1]  = SC_SUB_MESSAGE;
    rdm_response_msg[2]  = count;
    rdm_response_msg[3]  = rdm_msg[9];         // copy destination from received msg
    rdm_response_msg[4]  = rdm_msg[10];
    rdm_response_msg[5]  = rdm_msg[11];
    rdm_response_msg[6]  = rdm_msg[12];
    rdm_response_msg[7]  = rdm_msg[13];
    rdm_response_msg[8]  = rdm_msg[14];
    rdm_response_msg[9]  = ((MAN_ID >> 8) & 0xFF);
    rdm_response_msg[10] = ((MAN_ID >> 0) & 0xFF);
    rdm_response_msg[11] = ((my_unique_id >> 24) & 0xFF);
    rdm_response_msg[12] = ((my_unique_id >> 16) & 0xFF);
    rdm_response_msg[13] = ((my_unique_id >>  8) & 0xFF);
    rdm_response_msg[14] = ((my_unique_id >>  0) & 0xFF);
    rdm_response_msg[15] = rdm_msg[15];        // pass back Transaction Number
    rdm_response_msg[16] = RESPONSE_TYPE_ACK;
    rdm_response_msg[17] = rdm_message_count;
    rdm_response_msg[18] = 0;                  // Sub-Device MSB
    rdm_response_msg[19] = 0;                  // Sub-Device LSB
    rdm_response_msg[20] = command_class;
    rdm_response_msg[21] = rdm_msg[21];        // copy PID from received msg
    rdm_response_msg[22] = rdm_msg[22];
    rdm_response_msg[23] = data_len;
    uint16_t crc = calc_crc(rdm_response_msg, count);
    rdm_response_msg[count+0] = (crc >> 8) & 0xFF;
    rdm_response_msg[count+1] = (crc >> 0) & 0xFF;
    bsp_xmit_to_dmx_bus();
    Uart_XmitDmxSlotValues(rdm_response_msg, (count + 2),
                           break_time, mab_time, true);
    bsp_recv_from_dmx_bus();
}

static inline void send_disc_mute_response(uint8_t *rdm_msg)
{
    // no binding uid, only the control field
    rdm_response_msg[24] = 0;
    rdm_response_msg[25] = 0;
    send_rdm_msg(rdm_msg, DISCOVERY_COMMAND_RESPONSE, 2);
}

static inline void send_disc_un_mute_response(uint8_t *rdm_msg)
{
    send_disc_mute_response(rdm_msg);
}

static inline void send_get_device_info_response(uint8_t *rdm_msg)
{
    rdm_response_msg[24] = 0x01;  // RDM Protocol Version MSB
    rdm_response_msg[25] = 0x00;  // RDM Protocol Version LSB
    rdm_response_msg[26] = DEVICE_MODEL_ID1;
    rdm_response_msg[27] = DEVICE_MODEL_ID0;
    rdm_response_msg[28] = 0x01;  // Product Category MSB
    rdm_response_msg[29] = 0x01;  // Product Category LSB
    rdm_response_msg[30] = SW_VERSION_ID3;
    rdm_response_msg[31] = SW_VERSION_ID2;
    rdm_response_msg[32] = SW_VERSION_ID1;
    rdm_response_msg[33] = SW_VERSION_ID0;
    rdm_response_msg[34] = 0x00;  // DMX512 Footprint MSB
    rdm_response_msg[35] = DMX512_FOOTPRINT;
    rdm_response_msg[36] = 0x01;  // DMX512 Personality MSB
    rdm_response_msg[37] = 0x01;  // DMX512 Personality LSB
    rdm_response_msg[38] = (my_dmx_start_address >> 8) & 0xFF; // DMX512 Start Address MSB
    rdm_response_msg[39] = (my_dmx_start_address >> 0) & 0xFF; // DMX512 Start Address LSB
    rdm_response_msg[40] = 0x00;  // Sub-Device Count MSB
    rdm_response_msg[41] = 0x00;  // Sub-Device Count LSB
    rdm_response_msg[42] = 0x00;  // Sensor Count
    send_rdm_msg(rdm_msg, GET_COMMAND_RESPONSE, 19);
}

static inline void send_get_software_version_response(uint8_t *rdm_msg)
{
    rdm_response_msg[24] = 'N';
    rdm_response_msg[25] = 'X';
    rdm_response_msg[26] = 'P';
    rdm_response_msg[27] = ' ';
    rdm_response_msg[28] = 'L';
    rdm_response_msg[29] = 'P';
    rdm_response_msg[30] = 'C';
    rdm_response_msg[31] = '1';
    rdm_response_msg[32] = '1';
    rdm_response_msg[33] = '1';
    rdm_response_msg[34] = '4';
    rdm_response_msg[35] = ' ';
    rdm_response_msg[36] = 'D';
    rdm_response_msg[37] = 'M';
    rdm_response_msg[38] = 'X';
    rdm_response_msg[39] = '5';
    rdm_response_msg[40] = '1';
    rdm_response_msg[41] = '2';
    rdm_response_msg[42] = ' ';
    rdm_response_msg[43] = 's';
    rdm_response_msg[44] = 'l';
    rdm_response_msg[45] = 'a';
    rdm_response_msg[46] = 'v';
    rdm_response_msg[47] = 'e';
    send_rdm_msg(rdm_msg, GET_COMMAND_RESPONSE, 24);
}

static inline void send_get_dmx_start_address_response(uint8_t *rdm_msg)
{
    rdm_response_msg[24] = (my_dmx_start_address >> 8) & 0xFF;
    rdm_response_msg[25] = (my_dmx_start_address >> 0) & 0xFF;
    send_rdm_msg(rdm_msg, GET_COMMAND_RESPONSE, 2);
}

static inline void send_get_identify_device_response(uint8_t *rdm_msg, uint8_t state)
{
    rdm_response_msg[24] = state;
    send_rdm_msg(rdm_msg, GET_COMMAND_RESPONSE, 1);
}

static inline void send_set_dmx_start_address_response(uint8_t *rdm_msg)
{
    send_rdm_msg(rdm_msg, SET_COMMAND_RESPONSE, 0);
}

static inline void send_set_identify_device_response(uint8_t * rdm_msg)
{
    send_rdm_msg(rdm_msg, SET_COMMAND_RESPONSE, 0);
}

static inline bool check_broadcast(uint8_t * msg)
{
    if ((msg[5] == 0xFF) && (msg[6] == 0xFF) &&
        (msg[7] == 0xFF) && (msg[8] == 0xFF))
    {
        if (((msg[3] == 0xFF) && (msg[4] == 0xFF)) ||
            ((msg[3] == ((MAN_ID >> 8) & 0xFF)) &&
             (msg[4] == ((MAN_ID >> 0) & 0xFF))))
        {
            return true;
        }
    }
    return false;
}

static inline bool check_own_uid(uint8_t * msg)
{
    if ((msg[3] == ((MAN_ID >> 8) & 0xFF)) &&
        (msg[4] == ((MAN_ID >> 0) & 0xFF)) &&
        (msg[5] == ((my_unique_id >> 24) & 0xFF)) &&
        (msg[6] == ((my_unique_id >> 16) & 0xFF)) &&
        (msg[7] == ((my_unique_id >>  8) & 0xFF)) &&
        (msg[8] == ((my_unique_id >>  0) & 0xFF)))
    {
        return true;
    }
    return false;
}

static inline void handle_rdm_msg(uint8_t *rdm_msg)
{
static bool    rdm_muted = false;

    /* check sub start code in RDM Packet and minimum lenght */
    if ((rdm_msg[1] == SC_SUB_MESSAGE) && (rdm_msg[2] >= 24))
    {
        uint8_t  len = rdm_msg[2];
        uint16_t crc = ((uint16_t)rdm_msg[len+0] << 8) | rdm_msg[len+1];

        if (calc_crc(rdm_msg, len) != crc) return;
        
        bool broadcast = check_broadcast(rdm_msg);
        bool own_uid = check_own_uid(rdm_msg);
        
        if (!broadcast && !own_uid) return;

        uint16_t pid = ((uint16_t)rdm_msg[21] << 8) | rdm_msg[22];

        /* check command class (CC) */
        switch (rdm_msg[20])
        {
        case DISCOVERY_COMMAND:
            /* check parameter ID (PID) */
            if ((pid == DISC_UNIQUE_BRANCH) && (len == 36))
            {
                if (!rdm_muted)
                {
                    long long lower_bound_uid;
                    long long upper_bound_uid;
                
                    lower_bound_uid = rdm_msg[24];
                    lower_bound_uid <<= 8;
                    lower_bound_uid |= rdm_msg[25];
                    lower_bound_uid <<= 8;
                    lower_bound_uid |= rdm_msg[26];
                    lower_bound_uid <<= 8;
                    lower_bound_uid |= rdm_msg[27];
                    lower_bound_uid <<= 8;
                    lower_bound_uid |= rdm_msg[28];
                    lower_bound_uid <<= 8;
                    lower_bound_uid |= rdm_msg[29];
                
                    upper_bound_uid = rdm_msg[30];
                    upper_bound_uid <<= 8;
                    upper_bound_uid |= rdm_msg[31];
                    upper_bound_uid <<= 8;
                    upper_bound_uid |= rdm_msg[32];
                    upper_bound_uid <<= 8;
                    upper_bound_uid |= rdm_msg[33];
                    upper_bound_uid <<= 8;
                    upper_bound_uid |= rdm_msg[34];
                    upper_bound_uid <<= 8;
                    upper_bound_uid |= rdm_msg[35];

                    if ((my_uid >= lower_bound_uid) && (my_uid <= upper_bound_uid))
                    {
                        send_disc_response(rdm_msg);
                    }
                }
            }
            else if ((pid == DISC_MUTE) && (len == 24))
            {
                rdm_muted = true;
                if (own_uid) send_disc_mute_response(rdm_msg);
            }
            else if ((pid == DISC_UN_MUTE) && (len == 24))
            {
                rdm_muted = false;
                if (own_uid) send_disc_un_mute_response(rdm_msg);
            }
            break;
        case GET_COMMAND:
            if (own_uid)
            {
                if ((pid == DEVICE_INFO) && (len == 24))
                {
                    send_get_device_info_response(rdm_msg);
                }
                else if ((pid == SOFTWARE_VERSION_LABEL) && (len == 24))
                {
                    send_get_software_version_response(rdm_msg);
                }
                else if ((pid == DMX_START_ADDRESS) && (len == 24))
                {
                    send_get_dmx_start_address_response(rdm_msg);
                }
                else if ((pid == IDENTIFY_DEVICE) && (len == 24))
                {
                    send_get_identify_device_response(rdm_msg, identify_state);
                }
            }
            break;
        case SET_COMMAND:
            if ((pid == DMX_START_ADDRESS) && (len == 26))
            {
                my_dmx_start_address = ((uint16_t)rdm_msg[24] << 8) | rdm_msg[25];
                if (own_uid) send_set_dmx_start_address_response(rdm_msg);
            }
            else if ((pid == IDENTIFY_DEVICE) && (len == 25))
            {
                identify_state = rdm_msg[24];
                if (identify_state)
                {
                    dim_leds(255,255,255,255);
#ifdef SUPPORT_I2C_BUS
                    lcd_update("You found me :-)");
#endif
                }
                else
                {
                    dim_leds(0,0,0,0);
#ifdef SUPPORT_I2C_BUS
                    lcd_update("");
#endif
                }
                if (own_uid) send_set_identify_device_response(rdm_msg);
            }
            break;
        default:
            break;
        }
    }
}
#endif

/*******************************************************************************
 * global functions
 ******************************************************************************/

int main (void)
{
    bool     communication_lost = false;
    bool     not_addressed = false;
    uint16_t dmx_len;
    uint8_t *pslot_values;
    uint8_t  busyled_state = 0;
    
    SystemInit();
    bsp_init();
    
    bsp_control_led(0,0);    /* on startup turn on traffic led to check proper working */
    init_globals();
#ifdef SUPPORT_MANUAL_CONTROL
    init_manual_ctrl();
#endif
#ifdef SUPPORT_I2C_BUS
    lcd_update("NXP LPC1114         DMX512 slave");     /* startup msg on LCD */
#endif
    Uart_LoopBackTest(true);
    while(1)
    {
        // blocking read for 1000 milli seconds
        dmx_len = Uart_RecvDmxSlotValues(&pslot_values, true, 1000);
        if (dmx_len > 0)
        {
            if (communication_lost || not_addressed)
            {
                communication_lost = false;
                not_addressed = false;
#ifdef SUPPORT_I2C_BUS
                if (pslot_values[0] != SC_TEXT) lcd_update("");
#endif
            }
#ifndef VALIDATE_PERFORMANCE_ON_MAX_DMX_BUS_LOAD
            busyled_state ^= 1;
            bsp_control_led(0, busyled_state);
#endif
            /* Check the start code */
            if (pslot_values[0] == SC_NULL)
            {
#ifndef SUPPORT_RDM_OVER_DMX512
                my_dmx_start_address = get_my_slot_nrs();
#endif
                if (dmx_len > my_dmx_start_address + 0)
                {
                    uint8_t  mySlotValues[DMX512_FOOTPRINT] = {0,0,0,0};
                    mySlotValues[0] = pslot_values[my_dmx_start_address + 0];
                    if (dmx_len > my_dmx_start_address + 1)
                        mySlotValues[1] = pslot_values[my_dmx_start_address + 1];
                    if (dmx_len > my_dmx_start_address + 2)
                        mySlotValues[2] = pslot_values[my_dmx_start_address + 2];
                    if (dmx_len > my_dmx_start_address + 3)
                        mySlotValues[3] = pslot_values[my_dmx_start_address + 3];
                    dim_leds(mySlotValues[0], mySlotValues[1],
                             mySlotValues[2], mySlotValues[3]);
#ifdef VALIDATE_PERFORMANCE_ON_MAX_DMX_BUS_LOAD
                    bsp_control_led(0, ~mySlotValues[3]);
#endif
                }
                else
                {
                    not_addressed = true;
                    dim_leds(0,0,0,0);
#ifdef SUPPORT_I2C_BUS
                    lcd_update("Not addressed byDMX512 master");
#endif
                }
            }
#ifdef SUPPORT_I2C_BUS
            else if (pslot_values[0] == SC_TEXT)
            {   /* minimal length of msg must be 4 bytes(=slots) */
                if (dmx_len >= 4)
                {
                    lcd_update((char *)(pslot_values+3));
                }
            }
#endif
#ifdef SUPPORT_RDM_OVER_DMX512
            else if (pslot_values[0] == SC_RDM)
            {
                handle_rdm_msg(pslot_values);
            }
#endif
        }
        else
        {   /* nothing received in 1 second, switch off the leds */
            communication_lost = true;
#ifdef SUPPORT_RDM_OVER_DMX512
            if (!identify_state)
#endif
            {
                dim_leds(0,0,0,0);
#ifdef SUPPORT_I2C_BUS
                lcd_update("No packets from DMX512 master");
#endif
            }
        }
#ifdef SUPPORT_MANUAL_CONTROL
        check_manual_ctrl();
#endif
        WDT_Feed();
    }
}

void dim_leds(uint8_t value1, uint8_t value2, uint8_t value3, uint8_t value4)
{
    Timer_PWM_Set_Duty_Cycle(CT16B0, CTMAT0, value1);
    Timer_PWM_Set_Duty_Cycle(CT16B0, CTMAT1, value2);
    Timer_PWM_Set_Duty_Cycle(CT16B1, CTMAT0, value3);
    Timer_PWM_Set_Duty_Cycle(CT16B1, CTMAT1, value4);
}

void get_version_string(char *pstr)
{
#ifdef SUPPORT_MANUAL_CONTROL
    strcpy(pstr, FIRMWARE_VERSION);
#endif
}

/* EOF */
