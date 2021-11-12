#!/bin/sh
set -e

if [ "$TESTS_DIR" = "" ]; then
    echo "TESTS_DIR is not set"
    exit 1
fi

cat <<EOF

*
*  Starting integration tests...
*

EOF

failed=0
for test in $TESTS_DIR/*.test; do
    echo "==> $test"
    
    set +e
    sh $test
    result=$?
    set -e

    if [ $result -ne 0 ]; then
        echo "FAILED: exited with $result"
        failed=$(( $failed + 1 ))
    fi
done

if [ $failed -eq 0 ]; then
    cat <<EOF

*
*  Passed all integration tests!
*

EOF
else
    cat <<EOF

*
*  $failed tests are failed
*

EOF
fi