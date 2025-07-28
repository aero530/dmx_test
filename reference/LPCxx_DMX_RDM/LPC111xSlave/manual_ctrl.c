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
#include "wdt.h"
#include "lcd.h"
#include "main.h"
#include "manual_ctrl.h"

#ifdef SUPPORT_MANUAL_CONTROL

/*******************************************************************************
 * Types and defines
 ******************************************************************************/

#define KEY_NONE          0
#define KEY_UP            1
#define KEY_DOWN          2
#define KEY_LEFT          3
#define KEY_RIGHT         4
#define KEY_MIDDLE        5

#define TOP_MENU_LEVEL    0
#define SUB_MENU_LEVEL    1

#define TOP_MENU_ENTRY_1  1
#define TOP_MENU_ENTRY_2  2
#define TOP_MENU_ENTRY_3  3
#define TOP_MENU_ENTRY_4  4

#define SUB_MENU_ENTRY_1  1
#define SUB_MENU_ENTRY_2  2
#define SUB_MENU_ENTRY_3  3
#define SUB_MENU_ENTRY_4  4
#define SUB_MENU_ENTRY_5  5

#define MENU_ENTRY_0_0    "MANUAL CONTROL  "
#define MENU_ENTRY_0_1    "1 Go            "
#define MENU_ENTRY_0_2    "2 Set colors    "
#define MENU_ENTRY_0_3    "3 Version info  "
#define MENU_ENTRY_0_4    "4 Quit          "
#define MENU_0_ENTRIES    4

#define MENU_ENTRY_1_1    "1.1 Mode        "
#define MENU_ENTRY_1_2    "1.2 Speed       "
#define MENU_ENTRY_1_3    "1.3 Dim         "
#define MENU_ENTRY_1_4    "1.4 Quit        "
#define MENU_1_ENTRIES    4

#define MENU_ENTRY_2_1    "2.1 Color       "
#define MENU_ENTRY_2_2    "2.2 Hue         "
#define MENU_ENTRY_2_3    "2.3 Sat         "
#define MENU_ENTRY_2_4    "2.4 Bright      "
#define MENU_ENTRY_2_5    "2.5 Quit        "
#define MENU_2_ENTRIES    5

#define HUE_STEP_SIZE     5
#define SAT_STEP_SIZE     5
#define BRI_STEP_SIZE     5

#define NR_PRESET_COLORS  7

typedef struct
{
    uint16_t hue;          // 0 to 355 degrees
    uint8_t  saturation;   // 0 to 100 percent
    uint8_t  brightness;   // 0 to 100 percent
} color_t;

/*******************************************************************************
 * global variables
 ******************************************************************************/

static uint8_t  manual_mode;
static uint8_t  update_speed;
static uint8_t  brightness;
static uint8_t  color_index;
static color_t  preset_color[NR_PRESET_COLORS + 1] =
{
    {  0,   0,   0}, // not used
    {  0, 100, 100}, // preset color 1
    { 60, 100, 100},
    {120, 100, 100},
    {180, 100, 100},
    {240, 100, 100},
    {300, 100, 100},
    {355, 100, 100}  // preset color 7
};

static uint8_t  mm_counter_id;
static uint8_t  menu_level;
static uint8_t  top_menu_entry;
static uint8_t  sub_menu_entry;
static uint8_t  red_led_val;
static uint8_t  green_led_val;
static uint8_t  blue_led_val;
static uint8_t  running_color_index;
static uint16_t hue;
static uint32_t led_color_time;
static char     lcd_string[33];

/*******************************************************************************
 * local functions
 ******************************************************************************/

static void set_led_color_time(void)
{
    switch (update_speed)
    {
    case 1:  led_color_time = 24 * 60 * 60 * 1000;       break; // 1 day
    case 2:  led_color_time =      60 * 60 * 1000;       break; // 1 hour
    case 3:  led_color_time =       5 * 60 * 1000;       break; // 5 min
    case 4:  led_color_time =           60 * 1000;       break; // 1 min
    case 5:  led_color_time =           10 * 1000;       break; // 10 sec
    case 6:  led_color_time =            5 * 1000;       break; // 5 sec
    case 7:  led_color_time =                1000;       break; // 1 sec
    case 8:  led_color_time =                1000 / 5;   break; // 200ms  (5 Hz)
    case 9:  led_color_time =                1000 / 40;  break; // 25ms  (40 Hz)
    case 10: led_color_time =                1000 / 100; break; // 10ms (100 Hz)
    case 11: led_color_time =                1000 / 500; break; // 2 ms (500 Hz)
    case 12: led_color_time =                   0;       break; // continues
    default: led_color_time =                1000 / 40;  break;
    }
}

static void write_lcd_line2(char *pstr, uint8_t digits, uint16_t value)
{
    uint16_t i;
    
    strcpy(&lcd_string[16], pstr);
    if (digits != 0) {
        if (value >= 100) {
            i = value / 100;
            lcd_string[29] = '0' + i;
            value -= i * 100;
            lcd_string[30] = '0';
        }
        if (value >= 10) {
            i = value / 10;
            lcd_string[30] = '0' + i;
            value -= i * 10;
        }
        lcd_string[31] = '0' + value;
    }
    lcd_update(lcd_string);
}

static void hsb_to_rgb(uint16_t hue, uint8_t saturation, uint8_t brightness,
                       uint8_t * red, uint8_t * green, uint8_t * blue)
{
    float   fb = (float)brightness / (float)100.0;
    uint8_t ib = (uint8_t)(fb * (float)255.0);

    if (hue == 360) hue = 0;        
    if (saturation == 0) {
        // achromatic (grey)
        *red   = ib;
        *green = ib;
        *blue  = ib;
    } else {
        float   f, p, q, t;
        float   fs = (float)saturation / (float)100.0;
        uint8_t i = hue / 60;   // sector 0 to 5

	    f = (float)(hue - (i * 60)) / (float)60.0;   // factorial part of hue
	    p = fb * ((float)1.0 - fs);
	    q = fb * ((float)1.0 - fs * f);
	    t = fb * ((float)1.0 - fs * ((float)1.0 - f));
	    switch (i)
        {
		case 0:
			*red   = ib;
			*green = (uint8_t)(t * (float)255.0);
			*blue  = (uint8_t)(p * (float)255.0);
			break;
		case 1:
			*red   = (uint8_t)(q * (float)255.0);
			*green = ib;
			*blue  = (uint8_t)(p * (float)255.0);
			break;
		case 2:
			*red   = (uint8_t)(p * (float)255.0);
			*green = ib;
			*blue  = (uint8_t)(t * (float)255.0);
			break;
		case 3:
			*red   = (uint8_t)(p * (float)255.0);
			*green = (uint8_t)(q * (float)255.0);
			*blue  = ib;
			break;
		case 4:
			*red   = (uint8_t)(t * (float)255.0);
			*green = (uint8_t)(p * (float)255.0);
			*blue  = ib;
			break;
        case 5:
		default:
			*red   = ib;
			*green = (uint8_t)(p * (float)255.0);
			*blue  = (uint8_t)(q * (float)255.0);
			break;
	    }
    }
}

static uint8_t read_joystick(void)
{
static uint8_t prev_joystick_val = 0x1F;
    uint8_t cur_joystick_val = LPC_GPIO3->DATA & 0x1F;
    uint8_t ret_val = KEY_NONE;

    if (prev_joystick_val != cur_joystick_val)
    {
        if ((cur_joystick_val & 0x01) == 0) {
            ret_val = KEY_UP;
        } else if ((cur_joystick_val & 0x04) == 0) {
            ret_val = KEY_DOWN;
        } else if ((cur_joystick_val & 0x02) == 0) {
            ret_val = KEY_LEFT;
        } else if ((cur_joystick_val & 0x08) == 0) {
            ret_val = KEY_RIGHT;
        } else if ((cur_joystick_val & 0x10) == 0) {
            ret_val = KEY_MIDDLE;
        }
        prev_joystick_val = cur_joystick_val;
    }
    return ret_val;
}

static bool handle_menu(void)
{
    bool    ret_val = false;
    uint8_t key_press;
    
    key_press = read_joystick();
    switch (key_press)
    {
    case KEY_UP:
    case KEY_DOWN:
        if (menu_level == TOP_MENU_LEVEL) {
            if (key_press == KEY_UP) {
                top_menu_entry -= 1;
                if (top_menu_entry < 1) top_menu_entry = MENU_0_ENTRIES;
            } else {
                top_menu_entry += 1;
                if (top_menu_entry > MENU_0_ENTRIES) top_menu_entry = 1;
            }
            if (top_menu_entry == TOP_MENU_ENTRY_1) {
                write_lcd_line2(MENU_ENTRY_0_1, 0, 0);
            } else if (top_menu_entry == TOP_MENU_ENTRY_2) {
                write_lcd_line2(MENU_ENTRY_0_2, 0, 0);
            } else if (top_menu_entry == TOP_MENU_ENTRY_3) {
                write_lcd_line2(MENU_ENTRY_0_3, 0, 0);
            } else if (top_menu_entry == TOP_MENU_ENTRY_4) {
                write_lcd_line2(MENU_ENTRY_0_4, 0, 0);
            }
        } else if (menu_level == SUB_MENU_LEVEL) {
            if (top_menu_entry == TOP_MENU_ENTRY_1) {
                if (key_press == KEY_UP) {
                    sub_menu_entry -= 1;
                    if (sub_menu_entry < 1) sub_menu_entry = MENU_1_ENTRIES;
                } else {
                    sub_menu_entry += 1;
                    if (sub_menu_entry > MENU_1_ENTRIES) sub_menu_entry = 1;
                }
                if (sub_menu_entry == SUB_MENU_ENTRY_1) {
                    write_lcd_line2(MENU_ENTRY_1_1, 1, manual_mode);
                } else if (sub_menu_entry == SUB_MENU_ENTRY_2) {
                    write_lcd_line2(MENU_ENTRY_1_2, 2, update_speed);
                } else if (sub_menu_entry == SUB_MENU_ENTRY_3) {
                    write_lcd_line2(MENU_ENTRY_1_3, 3, brightness);
                } else if (sub_menu_entry == SUB_MENU_ENTRY_4) {
                    write_lcd_line2(MENU_ENTRY_1_4, 0, 0);
                }
            } else if (top_menu_entry == TOP_MENU_ENTRY_2) {
                if (key_press == KEY_UP) {
                    sub_menu_entry -= 1;
                    if (sub_menu_entry < 1) sub_menu_entry = MENU_2_ENTRIES;
                } else {
                    sub_menu_entry += 1;
                    if (sub_menu_entry > MENU_2_ENTRIES) sub_menu_entry = 1;
                }
                if (sub_menu_entry == SUB_MENU_ENTRY_1) {
                    write_lcd_line2(MENU_ENTRY_2_1, 1, color_index);
                } else if (sub_menu_entry == SUB_MENU_ENTRY_2) {
                    write_lcd_line2(MENU_ENTRY_2_2, 3, preset_color[color_index].hue);
                } else if (sub_menu_entry == SUB_MENU_ENTRY_3) {
                    write_lcd_line2(MENU_ENTRY_2_3, 3, preset_color[color_index].saturation);
                } else if (sub_menu_entry == SUB_MENU_ENTRY_4) {
                    write_lcd_line2(MENU_ENTRY_2_4, 3, preset_color[color_index].brightness);
                } else if (sub_menu_entry == SUB_MENU_ENTRY_5) {
                    write_lcd_line2(MENU_ENTRY_2_5, 0, 0);
                }
            }
        }
        break;
    case KEY_LEFT:
    case KEY_RIGHT:
        if (menu_level == SUB_MENU_LEVEL) {
            if (top_menu_entry == TOP_MENU_ENTRY_1) {
                if (sub_menu_entry == SUB_MENU_ENTRY_1) {
                    if (key_press == KEY_LEFT) {
                        manual_mode -= 1;
                        if (manual_mode < 1) manual_mode = 4;
                    } else {
                        manual_mode += 1;
                        if (manual_mode > 4) manual_mode = 1;
                    }
                    write_lcd_line2(MENU_ENTRY_1_1, 1, manual_mode);
                } else if (sub_menu_entry == SUB_MENU_ENTRY_2) {
                    if (key_press == KEY_LEFT) {
                        update_speed -= 1;
                        if (update_speed < 1) update_speed = 12;
                    } else {
                        update_speed += 1;
                        if (update_speed > 12) update_speed = 1;
                    }
                    write_lcd_line2(MENU_ENTRY_1_2, 2, update_speed);
                    set_led_color_time();
                } else if (sub_menu_entry == SUB_MENU_ENTRY_3) {
                    if (key_press == KEY_LEFT) {
                        if (brightness >= BRI_STEP_SIZE) {
                            brightness -= BRI_STEP_SIZE;
                        } else {
                            brightness = 100;
                        }
                    } else {
                        brightness += BRI_STEP_SIZE;
                        if (brightness > 100) brightness = 0;
                    }
                    write_lcd_line2(MENU_ENTRY_1_3, 3, brightness);
                }
            } else if (top_menu_entry == TOP_MENU_ENTRY_2) {
                if (sub_menu_entry == SUB_MENU_ENTRY_1) {
                    if (key_press == KEY_LEFT) {
                        color_index -= 1;
                        if (color_index < 1) color_index = NR_PRESET_COLORS;
                    } else {
                        color_index += 1;
                        if (color_index > NR_PRESET_COLORS) color_index = 1;
                    }
                    write_lcd_line2(MENU_ENTRY_2_1, 1, color_index);
                } else if (sub_menu_entry == SUB_MENU_ENTRY_2) {
                    if (key_press == KEY_LEFT) {
                        if (preset_color[color_index].hue >= HUE_STEP_SIZE) {
                            preset_color[color_index].hue -= HUE_STEP_SIZE;
                        } else {
                            preset_color[color_index].hue = 360 - HUE_STEP_SIZE;
                        }
                    } else {
                        preset_color[color_index].hue += HUE_STEP_SIZE;
                        if (preset_color[color_index].hue > (360 - HUE_STEP_SIZE))
                            preset_color[color_index].hue = 0;
                    }
                    write_lcd_line2(MENU_ENTRY_2_2, 3, preset_color[color_index].hue);
                } else if (sub_menu_entry == SUB_MENU_ENTRY_3) {
                    if (key_press == KEY_LEFT) {
                        if (preset_color[color_index].saturation >= SAT_STEP_SIZE) {
                            preset_color[color_index].saturation -= SAT_STEP_SIZE;
                        } else {
                            preset_color[color_index].saturation = 100;
                        }
                    } else {
                        preset_color[color_index].saturation += SAT_STEP_SIZE;
                        if (preset_color[color_index].saturation > 100)
                            preset_color[color_index].saturation = 0;
                    }
                    write_lcd_line2(MENU_ENTRY_2_3, 3, preset_color[color_index].saturation);
                } else if (sub_menu_entry == SUB_MENU_ENTRY_4) {
                    if (key_press == KEY_LEFT) {
                        if (preset_color[color_index].brightness >= BRI_STEP_SIZE) {
                            preset_color[color_index].brightness -= BRI_STEP_SIZE;
                        } else {
                            preset_color[color_index].brightness = 100;
                        }
                    } else {
                        preset_color[color_index].brightness += BRI_STEP_SIZE;
                        if (preset_color[color_index].brightness > 100)
                            preset_color[color_index].brightness = 0;
                    }
                    write_lcd_line2(MENU_ENTRY_2_4, 3, preset_color[color_index].brightness);
                }
                hsb_to_rgb(preset_color[color_index].hue,
                           preset_color[color_index].saturation,
                           preset_color[color_index].brightness,
                           &red_led_val, &green_led_val, &blue_led_val);
            }
        }
        break;
    case KEY_MIDDLE:
        if (menu_level == TOP_MENU_LEVEL) {
            if (top_menu_entry == TOP_MENU_ENTRY_1) {
                menu_level = SUB_MENU_LEVEL;
                sub_menu_entry = SUB_MENU_ENTRY_1;
                write_lcd_line2(MENU_ENTRY_1_1, 1, manual_mode);
                hsb_to_rgb(preset_color[color_index].hue,
                           preset_color[color_index].saturation,
                           preset_color[color_index].brightness,
                           &red_led_val, &green_led_val, &blue_led_val);
                hue = 0;
                running_color_index = color_index;
                bsp_reset_msec_cnt(mm_counter_id);
            } else if (top_menu_entry == TOP_MENU_ENTRY_2) {
                menu_level = SUB_MENU_LEVEL;
                sub_menu_entry = SUB_MENU_ENTRY_1;
                write_lcd_line2(MENU_ENTRY_2_1, 1, color_index);
                hsb_to_rgb(preset_color[color_index].hue,
                           preset_color[color_index].saturation,
                           preset_color[color_index].brightness,
                           &red_led_val, &green_led_val, &blue_led_val);
            } else if (top_menu_entry == TOP_MENU_ENTRY_3) {
                menu_level = SUB_MENU_LEVEL;
                sub_menu_entry = SUB_MENU_ENTRY_1;
                get_version_string(&lcd_string[16]);
                lcd_update(lcd_string);
            } else if (top_menu_entry == TOP_MENU_ENTRY_4) {
                ret_val = true;
            }
        }
        else if (menu_level == SUB_MENU_LEVEL) {
            if (top_menu_entry == TOP_MENU_ENTRY_1) {
                if (sub_menu_entry == MENU_1_ENTRIES) {
                    menu_level = TOP_MENU_LEVEL;
                    write_lcd_line2(MENU_ENTRY_0_1, 0, 0);
                    red_led_val   = 0;
                    green_led_val = 0;
                    blue_led_val  = 0;
                }
            } else if (top_menu_entry == TOP_MENU_ENTRY_2) {
                if (sub_menu_entry == MENU_2_ENTRIES) {
                    menu_level = TOP_MENU_LEVEL;
                    write_lcd_line2(MENU_ENTRY_0_2, 0, 0);
                    red_led_val   = 0;
                    green_led_val = 0;
                    blue_led_val  = 0;
                }
            } else if (top_menu_entry == TOP_MENU_ENTRY_3) {
                menu_level = TOP_MENU_LEVEL;
                write_lcd_line2(MENU_ENTRY_0_3, 0, 0);
            }
        }
        break;
    default:
        break;
    }
    return ret_val;
}

/*******************************************************************************
 * global functions
 ******************************************************************************/

void init_manual_ctrl(void)
{
    manual_mode   = 1;
    update_speed  = 10;
    color_index   = 1;
    brightness    = 100;
    mm_counter_id = bsp_claim_msec_cnt();
    set_led_color_time();
}

void check_manual_ctrl(void)
{
    if (read_joystick() == KEY_MIDDLE)
    {
        bool exit_manual_control = false;
        
        menu_level     = TOP_MENU_LEVEL;
        top_menu_entry = TOP_MENU_ENTRY_1;
        sub_menu_entry = SUB_MENU_ENTRY_1;
        red_led_val    = 0;
        green_led_val  = 0;
        blue_led_val   = 0;
        strcpy(&lcd_string[0], MENU_ENTRY_0_0);
        write_lcd_line2(MENU_ENTRY_0_1, 0, 0);
        while (!exit_manual_control)
        {
            dim_leds(red_led_val,green_led_val,blue_led_val,0);
            exit_manual_control = handle_menu();
            if ((top_menu_entry == TOP_MENU_ENTRY_1) && (menu_level == SUB_MENU_LEVEL)) {
                switch (manual_mode)
                {
                case 1: // Living colors
                    if (bsp_get_msec_cnt(mm_counter_id) > led_color_time) {
                        hue += 1;
                        if (hue >= 360) hue = 0;
                        hsb_to_rgb(hue, 100, brightness,
                                   &red_led_val, &green_led_val, &blue_led_val);
                        bsp_reset_msec_cnt(mm_counter_id);
                    }
                    break;
                case 2: // White
                    hsb_to_rgb(0, 0, brightness,
                               &red_led_val, &green_led_val, &blue_led_val);
                    break;
                case 3: // Jump over NR_OF_PRESET_COLORS colors
                    if (bsp_get_msec_cnt(mm_counter_id) > led_color_time) {
                        running_color_index += 1;
                        if (running_color_index > NR_PRESET_COLORS) {
                            running_color_index = 1;
                        }
                        hsb_to_rgb(preset_color[running_color_index].hue,
                                   preset_color[running_color_index].saturation,
                                   (uint8_t)(((float)preset_color[running_color_index].brightness * (float)brightness) / (float)100.0),
                                   &red_led_val, &green_led_val, &blue_led_val);
                        bsp_reset_msec_cnt(mm_counter_id);
                    }
                    break;
                case 4: // Fixed color output
                    hsb_to_rgb(preset_color[running_color_index].hue,
                               preset_color[running_color_index].saturation,
                               (uint8_t)(((float)preset_color[running_color_index].brightness * (float)brightness) / (float)100.0),
                               &red_led_val, &green_led_val, &blue_led_val);
                    break;
                default:
                    break;
                }
            }
            WDT_Feed();
        }
        lcd_update("");
        dim_leds(0,0,0,0);
    }
}

#endif

/* EOF */
