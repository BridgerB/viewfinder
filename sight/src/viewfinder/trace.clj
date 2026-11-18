(ns viewfinder.trace
  "Where a search spends its time, and how often the inner loop runs. Stage timers always run:
   a stage is seconds long, so a clock read costs nothing. Counters are off unless enabled,
   because the inner loop runs hundreds of millions of times."
  (:import [java.util.concurrent.atomic LongAdder]))

(set! *warn-on-reflection* true)

(def ^:private stages (atom []))

(defmacro timed
  "Runs body, records its wall time under stage, and returns body's value."
  [stage & body]
  `(let [t0# (System/nanoTime)
         v# (do ~@body)]
     (swap! @#'stages conj [~stage (/ (double (- (System/nanoTime) t0#)) 1e9)])
     v#))

(defn take-stages!
  "The stages recorded since the last call, as [stage seconds] in order, and clears them."
  []
  (let [[old _] (reset-vals! stages [])]
    old))

(def ^:const counting?
  "True when counters are on. Set once at startup with -Dviewfinder.count=true. It is a
   constant, inlined where bump is compiled, so a counter that is off is no code at all."
  (Boolean/getBoolean "viewfinder.count"))

(def ^:private counters
  {:fit (LongAdder.)
   :measure (LongAdder.)
   :second-tilt (LongAdder.)
   :reject-columns (LongAdder.)
   :reject-relief (LongAdder.)
   :reject-pitch (LongAdder.)
   :places (LongAdder.)
   :horizons (LongAdder.)})

(defmacro bump
  "Adds one to counter k when counting is on."
  [k]
  `(when counting?
     (.increment ^LongAdder (get @#'counters ~k))))

(defn take-counts!
  "Every counter's total since the last call, and resets them."
  []
  (into (sorted-map)
        (map (fn [[k ^LongAdder a]] [k (.sumThenReset a)]))
        counters))
