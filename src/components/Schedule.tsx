"use client";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ScheduleShow {
  title: string;
  time: string;
}

interface ScheduleData {
  schedule: {
    [key: string]: ScheduleShow[];
  };
}

export default function Schedule() {
  const [currentTime, setCurrentTime] = useState(new Date());
  const [schedule, setSchedule] = useState<ScheduleData | null>(null);
  const [nextAnime, setNextAnime] = useState<{
    title: string;
    time: Date;
  } | null>(null);

  useEffect(() => {
    loadSchedule();
    const timer = setInterval(() => {
      setCurrentTime(new Date());
      updateNextAnime();
    }, 60000); // Update every minute
    return () => clearInterval(timer);
  }, []);

  const loadSchedule = async () => {
    try {
      const data = await invoke<ScheduleData>("get_schedule");
      setSchedule(data);
      updateNextAnime(data);
    } catch (error) {
      console.error("Failed to load schedule:", error);
    }
  };

  const updateNextAnime = (data: ScheduleData | null = schedule) => {
    if (!data) return;

    const now = new Date();
    let nextShow: { title: string; time: Date } | null = null;

    Object.entries(data.schedule).forEach(([day, shows]) => {
      shows.forEach((show) => {
        const [hours, minutes] = show.time.split(":").map(Number);
        const showTime = new Date(now);
        showTime.setHours(hours, minutes, 0, 0);

        if (showTime > now && (!nextShow || showTime < nextShow.time)) {
          nextShow = { title: show.title, time: showTime };
        }
      });
    });

    setNextAnime(nextShow);
  };

  const formatCountdown = (target: Date) => {
    const diff = target.getTime() - currentTime.getTime();
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
    return `${hours}h ${minutes}m`;
  };

  if (!schedule) {
    return (
      <div className="text-center py-8 text-text-secondary">
        Loading schedule...
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto">
      {/* Header */}
      <div className="mb-6 space-y-2">
        <p className="text-text-primary font-medium">
          Current Time: {currentTime.toLocaleString()} UTC
        </p>
        {nextAnime && (
          <p className="text-primary font-bold">
            Next Episode: {nextAnime.title} at{" "}
            {nextAnime.time.toLocaleTimeString()} (in{" "}
            {formatCountdown(nextAnime.time)})
          </p>
        )}
      </div>

      {/* Schedule Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {Object.entries(schedule.schedule).map(([day, shows]) => (
          <div
            key={day}
            className="bg-surface rounded-lg border border-gray-200 overflow-hidden"
          >
            <div className="bg-primary text-white px-4 py-2 font-medium">
              {day.charAt(0).toUpperCase() + day.slice(1)}
            </div>
            <div className="p-4 space-y-2">
              {shows.map((show, index) => {
                const [hours, minutes] = show.time.split(":");
                const showTime = new Date(currentTime);
                showTime.setHours(parseInt(hours), parseInt(minutes), 0, 0);
                const isNext =
                  nextAnime &&
                  nextAnime.title === show.title &&
                  nextAnime.time.getTime() === showTime.getTime();

                return (
                  <div
                    key={`${show.title}-${index}`}
                    className={`p-2 rounded ${
                      isNext
                        ? "bg-blue-50 border-l-4 border-primary"
                        : "hover:bg-gray-50"
                    }`}
                  >
                    <div className="flex items-center space-x-2">
                      <span className="text-text-secondary font-medium">
                        {show.time}
                      </span>
                      <span
                        className={`flex-grow ${
                          isNext
                            ? "text-primary font-medium"
                            : "text-text-primary"
                        }`}
                      >
                        {show.title}
                      </span>
                      {isNext && (
                        <span className="text-primary text-sm font-medium">
                          Next
                        </span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
