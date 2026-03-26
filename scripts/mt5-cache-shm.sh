#!/bin/bash
# mt5-cache-shm.sh — Move MT5 BarCacheWriter DB to /dev/shm (RAM)
# All 3 MT5 instances share one DB. Persistent copy on SSD.
#
# Usage:
#   mt5-cache-shm.sh start   — Copy SSD → RAM, create symlinks
#   mt5-cache-shm.sh stop    — Flush RAM → SSD, remove symlinks
#   mt5-cache-shm.sh sync    — Flush RAM → SSD (periodic)

SHM_DB="/dev/shm/typhoon_mt5_cache.db"
SSD_DB="/home/typhoon/.config/typhoon-terminal/cache/typhoon_mt5_cache_persistent.db"

MT5_PATHS=(
    "/home/typhoon/.mt5_10/drive_c/Program Files/Darwinex MetaTrader 5/MQL5/Files/typhoon_mt5_cache.db"
    "/home/typhoon/.mt5_11/drive_c/Program Files/Darwinex MetaTrader 5/MQL5/Files/typhoon_mt5_cache.db"
    "/home/typhoon/.mt5_7/drive_c/Program Files/Darwinex MetaTrader 5/MQL5/Files/typhoon_mt5_cache.db"
)

case "$1" in
    start)
        echo "Loading MT5 cache to /dev/shm..."
        # Find the largest existing DB as the source of truth
        LARGEST=""
        LARGEST_SIZE=0
        for path in "${MT5_PATHS[@]}"; do
            if [ -f "$path" ] && [ ! -L "$path" ]; then
                size=$(stat -c%s "$path" 2>/dev/null || echo 0)
                if [ "$size" -gt "$LARGEST_SIZE" ]; then
                    LARGEST="$path"
                    LARGEST_SIZE=$size
                fi
            fi
        done

        # Use persistent SSD copy if it exists and is larger
        if [ -f "$SSD_DB" ]; then
            ssd_size=$(stat -c%s "$SSD_DB" 2>/dev/null || echo 0)
            if [ "$ssd_size" -gt "$LARGEST_SIZE" ]; then
                LARGEST="$SSD_DB"
                LARGEST_SIZE=$ssd_size
            fi
        fi

        if [ -n "$LARGEST" ]; then
            echo "Copying $(du -h "$LARGEST" | cut -f1) from $LARGEST → $SHM_DB"
            cp "$LARGEST" "$SHM_DB"
        else
            echo "No existing DB found, starting fresh"
            touch "$SHM_DB"
        fi

        # Create symlinks from all MT5 instances to the shared /dev/shm DB
        for path in "${MT5_PATHS[@]}"; do
            if [ -f "$path" ] && [ ! -L "$path" ]; then
                # Backup original
                mv "$path" "${path}.ssd_backup"
                echo "Backed up: $path"
            fi
            ln -sf "$SHM_DB" "$path"
            echo "Symlinked: $path → $SHM_DB"
        done
        echo "Done. MT5 BarCacheWriter now writes to RAM."
        ;;

    stop)
        echo "Flushing MT5 cache to SSD..."
        if [ -f "$SHM_DB" ]; then
            mkdir -p "$(dirname "$SSD_DB")"
            cp "$SHM_DB" "$SSD_DB"
            echo "Saved $(du -h "$SSD_DB" | cut -f1) to $SSD_DB"
        fi

        # Restore original files (remove symlinks)
        for path in "${MT5_PATHS[@]}"; do
            if [ -L "$path" ]; then
                rm "$path"
                if [ -f "${path}.ssd_backup" ]; then
                    mv "${path}.ssd_backup" "$path"
                    echo "Restored: $path"
                fi
            fi
        done
        echo "Done. Symlinks removed, SSD copy saved."
        ;;

    sync)
        # Periodic sync — call from cron or systemd timer
        if [ -f "$SHM_DB" ]; then
            mkdir -p "$(dirname "$SSD_DB")"
            cp "$SHM_DB" "$SSD_DB"
            echo "$(date +%H:%M:%S) Synced $(du -h "$SSD_DB" | cut -f1) to SSD"
        fi
        ;;

    *)
        echo "Usage: $0 {start|stop|sync}"
        exit 1
        ;;
esac
