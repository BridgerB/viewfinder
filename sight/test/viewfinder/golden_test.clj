(ns viewfinder.golden-test
  "The best camera at a fixed sample of real places, against a recorded answer. An exact speedup
   must reproduce it. Needs the gitignored elevation grid and field; skips without them. Run from
   the repository root. Delete golden.edn to record a new answer on purpose."
  (:require [clojure.edn :as edn]
            [clojure.java.io :as io]
            [clojure.pprint :as pp]
            [clojure.test :refer [deftest is]]
            [viewfinder.dem :as dem]
            [viewfinder.field :as field]
            [viewfinder.score :as score]
            [viewfinder.sky :as sky]))

(set! *warn-on-reflection* true)

(def ^:private grid-path "data/elevation/envi/n41w112")
(def ^:private field-path "data/elevation/field/n41w112.hzn")
(def ^:private image-path "static/sandy.jpg")
(def ^:private golden-path "sight/test/viewfinder/golden.edn")

(defn- answer
  "Best camera at 60 places drawn with a fixed seed from the 2 km field."
  []
  (let [grid (dem/read-envi grid-path)
        whole (field/read-field field-path)
        rng (java.util.Random. 7)
        picks (sort (take 60 (distinct (repeatedly #(.nextInt rng (int (:field/count whole)))))))
        part (field/subset whole picks)
        image (sky/read-image image-path)
        sampled (sky/decimate (sky/trace image) sky/columns)]
    (->> (score/surface part grid sampled (:sky/width image) (:sky/height image))
         (map (fn [p] (select-keys p [:place/lat :place/lon :place/yaw :place/fov :place/roll
                                      :place/pitch :place/rmse :place/loss])))
         (sort-by (juxt :place/lat :place/lon))
         vec)))

(deftest scoring-matches-the-recorded-answer
  (if-not (and (.exists (io/file (str grid-path ".hdr"))) (.exists (io/file field-path)))
    (println "golden: elevation data missing, skipped")
    (let [now (answer)]
      (if-not (.exists (io/file golden-path))
        (do (spit golden-path (with-out-str (pp/pprint now)))
            (println "golden: recorded" (count now) "places to" golden-path))
        (let [was (edn/read-string (slurp golden-path))]
          (is (= (count was) (count now)))
          (doseq [[a b] (map vector was now)]
            (is (= (select-keys a [:place/lat :place/lon :place/yaw :place/fov :place/roll])
                   (select-keys b [:place/lat :place/lon :place/yaw :place/fov :place/roll])))
            (doseq [k [:place/pitch :place/rmse :place/loss]]
              (is (< (Math/abs (- (double (k a)) (double (k b)))) 1e-9)
                  (str k " at " (:place/lat a) "," (:place/lon a))))))))))
