"use client";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ScheduleEntry {
  title: string;
  time: string;
  episode: number;
  air_date: string;
  day: string;
}

interface ScheduleData {
  [key: string]: ScheduleEntry[];
}

export default function Schedule() {
  const [currentTime, setCurrentTime] = useState<string>("");
  const [nextAnime, setNextAnime] = useState<string>("");
  const [schedule, setSchedule] = useState<ScheduleData>({});

  useEffect(() => {
    // Fetch schedule data
    const fetchSchedule = async () => {
      console.log("Fetching schedule data...");
      try {
        const entries: ScheduleEntry[] = await invoke("get_schedule");
        console.log("Raw schedule entries:", entries);

        // Group entries by day
        const groupedSchedule: ScheduleData = {};
        const days = [
          "Sunday",
          "Monday",
          "Tuesday",
          "Wednesday",
          "Thursday",
          "Friday",
          "Saturday",
        ];

        // Get current UTC time
        const now = new Date();
        let nextAnimeFound = false;

        entries.forEach((entry) => {
          const day = entry.day.charAt(0).toUpperCase() + entry.day.slice(1);
          const date = new Date(entry.air_date);

          if (!groupedSchedule[day]) {
            groupedSchedule[day] = [];
          }

          // Format the time in local timezone
          const localTime = date.toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
            hour12: false,
          });

          const showEntry = {
            ...entry,
            time: localTime,
          };

          groupedSchedule[day].push(showEntry);

          // Check if this is the next anime to air
          if (!nextAnimeFound && date > now) {
            const timeUntil = date.getTime() - now.getTime();
            const hours = Math.floor(timeUntil / (1000 * 60 * 60));
            const minutes = Math.floor(
              (timeUntil % (1000 * 60 * 60)) / (1000 * 60)
            );

            setNextAnime(
              `Next Episode: ${entry.title} at ${localTime} (in ${hours}h ${minutes}m)`
            );
            nextAnimeFound = true;
          }
        });

        // Sort shows within each day by time
        Object.keys(groupedSchedule).forEach((day) => {
          groupedSchedule[day].sort((a, b) => {
            return (
              new Date(a.air_date).getTime() - new Date(b.air_date).getTime()
            );
          });
        });

        console.log("Final grouped schedule:", groupedSchedule);
        setSchedule(groupedSchedule);
      } catch (error) {
        console.error("Failed to fetch schedule:", error);
        console.error("Error details:", error);
      }
    };

    console.log("Setting up schedule refresh...");
    // Update clock and schedule every minute
    const timer = setInterval(() => {
      const now = new Date();
      console.log("Updating time:", now);
      setCurrentTime(now.toUTCString());
      updateNextAnime(now);
      fetchSchedule(); // Refresh schedule data
    }, 60000); // Every minute

    // Initial fetch
    fetchSchedule();
    setCurrentTime(new Date().toUTCString());

    return () => {
      console.log("Cleaning up schedule component");
      clearInterval(timer);
    };
  }, []);

  const updateNextAnime = (now: Date) => {
    let nextShow: ScheduleEntry | null = null;
    let earliestTime = Infinity;

    Object.values(schedule).forEach((shows) => {
      shows.forEach((show) => {
        const airDate = new Date(show.air_date);
        const timeDiff = airDate.getTime() - now.getTime();

        if (timeDiff > 0 && timeDiff < earliestTime) {
          earliestTime = timeDiff;
          nextShow = show;
        }
      });
    });

    if (nextShow) {
      const airDate = new Date(nextShow.air_date);
      const timeUntil = airDate.getTime() - now.getTime();
      const hours = Math.floor(timeUntil / (1000 * 60 * 60));
      const minutes = Math.floor((timeUntil % (1000 * 60 * 60)) / (1000 * 60));

      setNextAnime(
        `Next Episode: ${
          nextShow.title
        } at ${airDate.toLocaleTimeString()} (in ${hours}h ${minutes}m)`
      );
    }
  };

  return (
    <div className="max-w-4xl mx-auto">
      {/* Current Time */}
      <div className="mb-4">
        <p className="text-text-primary font-bold">
          Current Time: {currentTime}
        </p>
      </div>

      {/* Next Anime */}
      <div className="mb-6">
        <p className="text-text-primary font-bold">{nextAnime}</p>
      </div>

      {/* Schedule */}
      <div className="bg-white rounded-lg shadow">
        {Object.entries(schedule).map(([day, shows]) => (
          <div
            key={day}
            className="p-4 border-b border-gray-200 last:border-b-0"
          >
            <h3 className="text-lg font-bold text-text-primary mb-4 pb-2 border-b-2 border-primary">
              {day}
            </h3>
            <div className="space-y-2">
              {shows.map((show, index) => (
                <div
                  key={index}
                  className="p-2 bg-gray-50 rounded hover:bg-gray-100 transition-colors"
                >
                  <span className="text-gray-600 font-medium">
                    {show.time} UTC
                  </span>
                  {" - "}
                  <span className="text-text-primary">{show.title}</span>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
