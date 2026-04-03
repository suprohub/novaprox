#!/bin/sh
set -e

MAX_RETRIES=3
RETRY_DELAY=10

cp /usr/local/sources.txt sources.txt

get_random_message() {
    messages="\
Ркн говно 👨‍🦽
Свежачок) 🍓
Последний фикс перед обновой ⚒️
Новые ускорители 🚀
Синхронизация успешна 📈
Вот это уж точно работает! ♻️
Вот бы это все закончилось 🥱"
    count=$(printf "%s\n" "$messages" | wc -l)
    idx=$(( ($RANDOM % count) + 1 ))
    printf "%s\n" "$messages" | sed -n "${idx}p"
}

while true; do
    echo "=== $(date) ==="

    novaprox -o vless.txt

    retry=0
    while [ $retry -lt $MAX_RETRIES ]; do
        set -- commit --token "$GIT_TOKEN" --repo "$GIT_REPO" --author-name "${GIT_USER:-proxy-checker}" --author-email "${GIT_EMAIL:-proxy-checker@localhost}" --message "$(get_random_message)" vless.txt
        if [ -n "$GIT_BRANCH" ]; then
            set -- "$@" --branch "$GIT_BRANCH"
        fi
        if ghcp "$@"; then
            break
        fi
        retry=$((retry+1))
        echo "Commit failed, retry $retry/$MAX_RETRIES in ${RETRY_DELAY}s..."
        sleep $RETRY_DELAY
    done
    if [ $retry -eq $MAX_RETRIES ]; then
        echo "Commit failed after $MAX_RETRIES attempts"
    else
        echo "Changes committed successfully"
    fi

    current_timestamp=$(date +%s)
    hour=$(date +%H)
    min=$(date +%M)
    sec=$(date +%S)

    hour_dec=$((10#$hour))
    min_dec=$((10#$min))
    sec_dec=$((10#$sec))

    current_day_seconds=$((hour_dec * 3600 + min_dec * 60 + sec_dec))

    if [ $hour_dec -ge 0 ] && [ $hour_dec -lt 6 ]; then
        if [ $hour_dec -eq 5 ]; then
            next_hour=6
        else
            next_hour=$((hour_dec + 1))
        fi
        next_min=0
        next_sec=0
        next_day_seconds=$((next_hour * 3600 + next_min * 60 + next_sec))
        if [ $next_day_seconds -le $current_day_seconds ]; then
            next_day_seconds=$((next_day_seconds + 24*3600))
        fi
        delay=$((next_day_seconds - current_day_seconds))
    else
        if [ $min_dec -lt 20 ]; then
            next_min=20
            next_hour=$hour_dec
        elif [ $min_dec -lt 40 ]; then
            next_min=40
            next_hour=$hour_dec
        else
            next_min=0
            next_hour=$((hour_dec + 1))
        fi
        next_sec=0
        next_day_seconds=$((next_hour * 3600 + next_min * 60 + next_sec))
        if [ $next_day_seconds -le $current_day_seconds ]; then
            next_day_seconds=$((next_day_seconds + 24*3600))
        fi
        delay=$((next_day_seconds - current_day_seconds))
    fi

    echo "Sleeping for $delay seconds until next scheduled time..."
    sleep $delay
done