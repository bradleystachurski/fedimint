#!/usr/bin/env bash

FM_DEVIMINT_DISABLE_MODULE_LNV2=1 just test-upgrades v0.3.3 current &> results.log
sleep 60
FM_DEVIMINT_DISABLE_MODULE_LNV2=1 just test-upgrades v0.3.4-rc.1 current &>> results.log
sleep 60
FM_DEVIMINT_DISABLE_MODULE_LNV2=1 just test-upgrades v0.4.0 current &>> results.log
sleep 60
FM_DEVIMINT_DISABLE_MODULE_LNV2=1 just test-upgrades v0.4.1 current &>> results.log
sleep 60
FM_DEVIMINT_DISABLE_MODULE_LNV2=1 just test-upgrades v0.4.2 current &>> results.log
sleep 60
FM_DEVIMINT_DISABLE_MODULE_LNV2=1 just test-upgrades v0.4.3 current &>> results.log
sleep 60
FM_DEVIMINT_DISABLE_MODULE_LNV2=1 just test-upgrades v0.4.4 current &>> results.log
sleep 60
FM_DEVIMINT_DISABLE_MODULE_LNV2=1 just test-upgrades v0.3.3 v0.4.0 current &>> results.log
sleep 60
FM_DEVIMINT_DISABLE_MODULE_LNV2=1 TEST_KINDS=fedimint-cli,gateway just test-upgrades v0.3.1 current &>> results.log
