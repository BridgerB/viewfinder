(ns viewfinder.score-test
  "The camera geometry and the per-yaw measure on synthetic data. No elevation data needed."
  (:require [clojure.test :refer [deftest is testing]]
            [viewfinder.horizon :as horizon]
            [viewfinder.score :as score]))

(set! *warn-on-reflection* true)

(def ^:private ray #'score/ray)
(def ^:private measure #'score/measure)

(defn- close?
  [a b tolerance]
  (<= (Math/abs (- (double a) (double b))) (double tolerance)))

(deftest outline-inverts-ray
  (testing "a horizon drawn through a camera lands on the pixels that camera maps back to it"
    (doseq [fov [40.0 70.0 100.0]
            roll [-2.0 0.0 2.0]
            pitch [-3.0 0.0 12.0]]
      (let [width 2000
            height 1500
            angles (short-array 360)
            _ (dotimes [az 360]
                (aset angles az (short (Math/round (* 100.0 (+ 3.0 (* 2.0 (Math/sin (Math/toRadians (* 3.0 az))))))))))
            place {:place/yaw 120.0 :place/fov fov :place/roll roll :place/pitch pitch}
            f (/ width 2.0 (Math/tan (Math/toRadians (/ fov 2.0))))
            pixels (score/outline {:horizon/angles angles} place width height)]
        (is (seq pixels))
        (doseq [[x y] pixels]
          (let [[offset elevation] (ray width height f roll pitch x y)
                az (+ 120.0 (double offset))
                whole (Math/round az)
                expected (/ (aget angles (int (mod whole 360))) 100.0)]
            (is (close? az whole 1e-6)
                (str "outline points sit on whole degrees, fov " fov " roll " roll " pitch " pitch))
            (is (close? elevation expected 1e-6)
                (str "elevation round-trips, fov " fov " roll " roll " pitch " pitch))))))))

(defn- run-measure
  "measure on one synthetic window: horizon angles h (degrees) at azimuths yaw + offsets, and
   the photograph's geometric elevations geoms. Returns the slot as a map, or nil."
  [offsets geoms horizon-fn]
  (let [n (count offsets)
        angles (short-array 360)
        distances (float-array 360 5000.0)
        _ (dotimes [az 360] (aset angles az (short (Math/round (* 100.0 (double (horizon-fn az)))))))
        slot (double-array 7)
        buf #(double-array n)
        ok (measure angles distances 0 360 (double-array offsets) (double-array geoms) 0
                    (buf) (buf) (buf) (buf) (buf) (buf) slot)]
    (when ok
      (zipmap [:rmse :pitch :used :gsigma :hsigma :corr :loss] (vec slot)))))

(deftest measure-pitch-absorbs-a-constant-offset
  (let [offsets (mapv double (range -40 40))
        shape (fn [az] (+ 4.0 (* 2.0 (Math/sin (Math/toRadians (* 6.0 az))))))
        horizon-fn (fn [az] (shape (if (> az 180) (- az 360) az)))
        geoms (mapv (fn [o] (- (horizon-fn (long (mod o 360))) 1.5)) offsets)
        hit (run-measure offsets geoms horizon-fn)]
    (is hit)
    (is (close? (:pitch hit) 1.5 0.02) "the whole gap is pitch")
    (is (close? (:rmse hit) 0.0 0.02) "and nothing is left over")
    (is (close? (:corr hit) 1.0 1e-3))))

(deftest measure-rejects-a-flat-photograph
  (let [offsets (mapv double (range -40 40))
        hit (run-measure offsets (vec (repeat 80 2.0)) (fn [_] 2.0))]
    (is (nil? hit) "a photograph with no relief cannot place a camera")))

(deftest select-agrees-with-sort
  (let [select #'score/select
        rng (java.util.Random. 3)]
    (dotimes [_ 500]
      (let [n (+ 1 (.nextInt rng 200))
            values (double-array (repeatedly n #(Math/floor (* 20.0 (.nextGaussian rng)))))
            sorted (doto (aclone values) (java.util.Arrays/sort))
            k (.nextInt rng n)]
        (is (= (aget sorted k) (select (aclone values) n k))
            (str "k " k " of " n))))))

(deftest dot-matches-a-plain-sum
  (let [dot #'score/dot
        rng (java.util.Random. 5)]
    (dotimes [_ 200]
      (let [m (+ 1 (.nextInt rng 130))
            b (.nextInt rng 300)
            g (double-array (repeatedly m #(.nextGaussian rng)))
            e (double-array (repeatedly 720 #(.nextGaussian rng)))
            plain (reduce + (map (fn [k] (* (aget g k) (aget e (+ b k)))) (range m)))]
        (is (close? plain (dot g e b) 1e-9) (str "m " m " b " b))))))

(deftest missing-is-not-a-real-angle
  (is (= horizon/missing Short/MIN_VALUE)))
