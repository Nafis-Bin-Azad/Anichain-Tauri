"use client";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ScheduleEntry {
  title: string;
  time: string;
  episode: number;
  air_date: string;
  day: string;
  isReleased?: boolean;
}

interface ScheduleData {
  [key: string]: ScheduleEntry[];
}

export default function Schedule() {
  const [currentTime, setCurrentTime] = useState<string>("");
  const [nextAnime, setNextAnime] = useState<string>("");
  const [schedule, setSchedule] = useState<ScheduleData>({});
  const [todaySchedule, setTodaySchedule] = useState<ScheduleEntry[]>([]);
  const [clockTime, setClockTime] = useState<string>("");

  useEffect(() => {
    const fetchSchedule = async () => {
      try {
        const entries: ScheduleEntry[] = await invoke("get_schedule");

        // Get current UTC time
        const now = new Date();
        let nextAnimeFound = false;

        // Group entries by day
        const groupedSchedule: ScheduleData = {};

        // Get today's schedule
        const today = now.toLocaleDateString("en-US", { weekday: "long" });
        const todayShows: ScheduleEntry[] = [];

        entries.forEach((entry) => {
          const date = new Date(entry.air_date);
          const day = entry.day.charAt(0).toUpperCase() + entry.day.slice(1);

          if (!groupedSchedule[day]) {
            groupedSchedule[day] = [];
          }

          const localTime = date.toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
            hour12: false,
          });

          const isReleased = date < now;
          const showEntry = {
            ...entry,
            time: localTime,
            isReleased,
          };

          groupedSchedule[day].push(showEntry);

          // Add to today's schedule if it's today
          if (day === today) {
            todayShows.push(showEntry);
          }

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

        setSchedule(groupedSchedule);
        setTodaySchedule(todayShows);

        // Format current time
        const formattedDate = now.toLocaleDateString("en-US", {
          month: "long",
          day: "numeric",
          year: "numeric",
        });
        const formattedTime = now.toLocaleTimeString("en-US", {
          hour: "2-digit",
          minute: "2-digit",
          second: "2-digit",
          hour12: true,
        });
        setCurrentTime(`${formattedDate}, ${formattedTime}`);
      } catch (error) {
        console.error("Failed to fetch schedule:", error);
      }
    };

    // Initial fetch
    fetchSchedule();

    // Update every minute
    const timer = setInterval(fetchSchedule, 60000);

    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    // Function to update clock
    const updateClock = () => {
      const now = new Date();
      const formattedTime = now.toLocaleTimeString("en-US", {
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
        hour12: true,
      });
      setClockTime(formattedTime);
    };

    // Update clock immediately and every second
    updateClock();
    const clockTimer = setInterval(updateClock, 1000);

    return () => clearInterval(clockTimer);
  }, []);

  return (
    <div className="max-w-4xl mx-auto">
      {/* Today's Schedule with real-time clock */}
      <div className="mb-6 bg-white rounded-lg shadow-md overflow-hidden">
        <div className="bg-primary p-4 border-b border-gray-200">
          <div className="flex justify-between items-center">
            <h2 className="text-lg font-bold text-white">Airtime today</h2>
            <div className="flex items-center space-x-2">
              <span className="text-white text-sm opacity-80">
                Current time:
              </span>
              <span className="text-white font-medium">{clockTime}</span>
            </div>
          </div>
        </div>
        <div className="p-4">
          <div className="space-y-2">
            {todaySchedule.map((show, index) => (
              <div
                key={index}
                className="flex items-center space-x-2 p-3 bg-gray-50 rounded-lg border border-gray-100 hover:bg-gray-100 transition-colors"
              >
                <span className="text-gray-600 font-medium min-w-[80px] border-r border-gray-200 pr-3">
                  {show.time}
                </span>
                <span className="text-text-primary flex-grow pl-3">
                  {show.title}
                </span>
                {show.isReleased && (
                  <span className="text-green-500 bg-green-50 p-1 rounded-full w-6 h-6 flex items-center justify-center">
                    ✓
                  </span>
                )}
              </div>
            ))}
            {todaySchedule.length === 0 && (
              <div className="text-center text-gray-500 py-4">
                No shows scheduled for today
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Next Anime */}
      <div className="mb-6">
        <p className="text-text-primary font-bold">{nextAnime}</p>
      </div>

      {/* Full Schedule */}
      <div className="bg-white rounded-lg shadow">
        <h2 className="text-lg font-bold p-4 border-b border-gray-200">
          Weekly Schedule
        </h2>
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
                  className="flex items-center space-x-2 p-2 bg-gray-50 rounded hover:bg-gray-100 transition-colors"
                >
                  <span className="text-gray-600 font-medium min-w-[80px]">
                    {show.time}
                  </span>
                  <span className="text-text-primary flex-grow">
                    {show.title}
                  </span>
                  {show.isReleased && <span className="text-green-500">✓</span>}
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
