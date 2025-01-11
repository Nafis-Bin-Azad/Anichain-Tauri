"use client";

import { useState, useEffect } from "react";

interface ScheduleItem {
  title: string;
  time: string;
}

interface ScheduleData {
  [key: string]: ScheduleItem[];
}

export default function Schedule() {
  const [currentTime, setCurrentTime] = useState<string>("");
  const [nextAnime, setNextAnime] = useState<string>("");
  const [schedule, setSchedule] = useState<ScheduleData>({
    Monday: [
      { title: "Jujutsu Kaisen Season 2", time: "17:00" },
      { title: "Solo Leveling", time: "18:30" },
    ],
    Tuesday: [{ title: "Demon Slayer", time: "17:00" }],
    // Add more days as needed
  });

  useEffect(() => {
    // Update clock every second
    const timer = setInterval(() => {
      const now = new Date();
      setCurrentTime(now.toUTCString());
      updateNextAnime(now);
    }, 1000);

    return () => clearInterval(timer);
  }, []);

  const updateNextAnime = (now: Date) => {
    let nextShow: { title: string; time: Date } | null = null;

    Object.entries(schedule).forEach(([day, shows]) => {
      shows.forEach((show) => {
        const [hours, minutes] = show.time.split(":").map(Number);
        const showTime = new Date(now);
        showTime.setUTCHours(hours, minutes, 0, 0);

        if (showTime > now && (!nextShow || showTime < nextShow.time)) {
          nextShow = { title: show.title, time: showTime };
        }
      });
    });

    if (nextShow) {
      const timeUntil = nextShow.time.getTime() - now.getTime();
      const hours = Math.floor(timeUntil / (1000 * 60 * 60));
      const minutes = Math.floor((timeUntil % (1000 * 60 * 60)) / (1000 * 60));
      setNextAnime(
        `Next Episode: ${
          nextShow.title
        } at ${nextShow.time.toUTCString()} (in ${hours}h ${minutes}m)`
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
