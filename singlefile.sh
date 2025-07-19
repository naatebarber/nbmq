#!/bin/bash
rm -rf sf.txt; touch sf.txt; for i in `find src/ -name "*.rs"`; do cat $i >> sf.txt; done
