(ns viewfinder.score
  "Which packed place could have taken the photograph. The photo is a window of some field of
   view, some roll and some tilt. Yaw is tried at every degree. A pixel is a direction in space:
   roll about the optical axis, then tilt, then yaw. Away from the middle of a wide frame a
   tilted camera's rows are not lines of equal elevation, so the tilt is part of the projection
   and only the last fraction of a degree is left to the mean residual.
   The rank is a loss in degrees. The trimmed error, the log gap between the horizon's relief
   and the photograph's once that gap exceeds one step of the field-of-view grid, and twice the
   correlation miss. Three penalties sit on top of that. A window that is mostly the slope under
   the camera, a window that takes its skyline from the walk's first step, and a look direction
   where the ground ahead falls away. Dividing the error by the relief made a tall wrong ridge
   look as close as the real one. The field of view runs from a phone's main lens to its
   ultrawide. A narrower lens shrinks a farther copy of the ridge until the relief matches. A
   pole is dropped before the sums."
  (:require [clojure.math :as math]
            [viewfinder.dem :as dem]
            [viewfinder.field :as field]
            [viewfinder.horizon :as horizon]
            [viewfinder.trace :as trace]
            [viewfinder.wave :as wave])
  (:import [java.util Arrays]))

(set! *warn-on-reflection* true)
(set! *unchecked-math* :warn-on-boxed)

(def ^:dynamic *fovs* [40.0 45.0 50.0 55.0 60.0 65.0 70.0 75.0 80.0 85.0 90.0 95.0 100.0 105.0 110.0 115.0]) ; a 50 mm lens through a phone's ultrawide. A camera whose lens is outside the list is placed nearer or farther to make up the difference
(def rolls [-2.0 0.0 2.0])
(def ref-fov 70.0) ; degrees. The trimmed error is counted as if the frame spanned this much: a trace is off by pixels, and a narrow lens makes a pixel a smaller angle
(def pitch-low -5) ; degrees. A skyline photograph is not aimed at the ground
(def pitch-high 30) ; degrees. Past this the frame is sky
(def pitch-slack 1.0) ; degrees the mean residual may absorb. More than this bends the rows of a wide frame
(def keep-fraction 0.98) ; a pole or a tree, not a tenth of the ridge: the saddle is the signal
(def min-relief 0.2) ; degrees. Below this the photograph is a flat line and cannot place a camera
(def near-m 2000.0) ; closer than this, once it fills the frame, is the ground the camera stands on
(def near-keep 0.20) ; a foothill may cover this much of the window and still be the photograph
(def near-weight 0.0) ; degrees added when the whole remainder of the window is that near ground. Off: at 1.5 it sent a canyon-mouth photo 51 km away, and with the 3D projection and pixel error no valley photo needs it
(def floor-m 500.0) ; a hit this close is the walk's first step, the berm beside the camera
(def floor-weight 2.0) ; degrees if that berm is the whole window
(def foreground-m 1200.0) ; a valley photograph is still on the flat at this distance
(def drop-allow-m 40.0) ; a field is not perfectly level
(def drop-scale-m 400.0) ; a fall of this many meters past the allowance costs one degree
(def top-k 16)

(defn- ray
  "Azimuth offset from the optical axis and elevation above the level, in degrees, of pixel x y.
   f is the focal length in pixels. roll and pitch are degrees. The pixel is the direction
   right, up, forward. Roll turns it about forward and pitch lifts it about right. A column at
   the edge of a wide frame is farther from the camera than f, so its elevation is smaller than
   its row alone says."
  [width height f roll pitch x y]
  (let [rx (- (double x) (/ (double width) 2.0))
        ry (- (/ (double height) 2.0) (double y))
        f (double f)
        c (math/cos (math/to-radians roll))
        s (math/sin (math/to-radians roll))
        xr (- (* rx c) (* ry s))
        yr (+ (* rx s) (* ry c))
        cp (math/cos (math/to-radians pitch))
        sp (math/sin (math/to-radians pitch))
        up (+ (* yr cp) (* f sp))
        forward (- (* f cp) (* yr sp))]
    [(math/to-degrees (math/atan2 xr forward))
     (math/to-degrees (math/atan2 up (math/hypot xr forward)))]))

(defn- camera
  "The skyline projected through one field of view, roll and pitch: parallel arrays of azimuth
   offset and elevation."
  [width height fov roll pitch skyline]
  (let [n (count skyline)
        f (/ (double width) 2.0 (math/tan (math/to-radians (/ (double fov) 2.0))))
        offsets (double-array n)
        geoms (double-array n)]
    (dotimes [i n]
      (let [{:sky/keys [x y]} (nth skyline i)
            [offset geom] (ray width height f roll pitch x y)]
        (aset offsets i (double offset))
        (aset geoms i (double geom))))
    {:camera/fov (double fov)
     :camera/roll (double roll)
     :camera/pitch (double pitch)
     :camera/offsets offsets
     :camera/geoms geoms}))

(defn- lens
  "One field of view and roll, projected at every whole degree of pitch. :lens/level is the
   camera with no pitch. :lens/kappa is the mean cosine of its azimuth offsets: lifting the
   camera a degree lifts a column that far off the axis by its cosine of a degree."
  [width height fov roll skyline]
  (let [tilts (mapv (fn [pitch] (camera width height fov roll pitch skyline))
                    (range pitch-low (inc (long pitch-high))))
        level (nth tilts (- (long pitch-low)))
        ^doubles offsets (:camera/offsets level)
        n (alength offsets)
        kappa (loop [i 0 sum 0.0]
                (if (== i n)
                  (/ sum (double (max 1 n)))
                  (recur (inc i) (+ sum (math/cos (math/to-radians (aget offsets i)))))))]
    {:lens/tilts tilts
     :lens/level level
     :lens/kappa kappa}))

(defn- lenses
  "Every field of view and roll, each already projected at every pitch. Without fovs and rolls,
   the full search: *fovs* and rolls."
  ([width height skyline] (lenses width height skyline *fovs* rolls))
  ([width height skyline fovs rolls]
   (into []
         (for [fov fovs
               roll rolls]
           (lens width height fov roll skyline)))))

(def all-yaws
  "Every whole degree."
  (int-array (range 360)))

(defn yaws-every
  "Every step-th whole degree."
  ^ints [step]
  (int-array (range 0 360 step)))

(defn yaws-around
  "Whole degrees within reach of center, wrapped."
  ^ints [center reach]
  (let [c (long (Math/round (double center)))
        reach (long reach)]
    (int-array (map (fn [d] (mod (+ c (long d)) 360)) (range (- reach) (inc reach))))))

(defn- wrap
  "Azimuth into [0, 360). One step is enough: yaw is a whole degree and the offset is a field
   of view."
  ^double [^double azimuth]
  (cond
    (>= azimuth 360.0) (- azimuth 360.0)
    (neg? azimuth) (+ azimuth 360.0)
    :else azimuth))

(defn- angle-at
  "Centidegrees of the horizon at azimuth, interpolated between the two surrounding degrees.
   NaN when either sample is missing."
  ^double [^shorts angles ^long base ^long azimuths ^double azimuth]
  (let [i0 (long azimuth)
        i1 (if (== i0 (dec azimuths)) 0 (inc i0))
        t (- azimuth (double i0))
        h0 (long (aget angles (int (+ base i0))))
        h1 (long (aget angles (int (+ base i1))))]
    (if (or (== h0 (long horizon/missing)) (== h1 (long horizon/missing)))
      Double/NaN
      (+ (double h0) (* t (- (double h1) (double h0)))))))

(defn- split-at-pivot
  "Partitions a[lo..hi] around the median of its first, middle and last values. Returns where
   the pivot lands: everything before it is smaller, everything after is not. One loop in tail
   position, so it compiles to a primitive loop."
  ^long [^doubles a ^long lo ^long hi]
  (let [mid (unsigned-bit-shift-right (+ lo hi) 1)
        x (aget a lo)
        y (aget a mid)
        z (aget a hi)
        at (if (< x y)
             (if (< y z) mid (if (< x z) hi lo))
             (if (< x z) lo (if (< y z) hi mid)))
        pivot (aget a at)]
    (aset a at (aget a hi))
    (aset a hi pivot)
    (loop [i lo store lo]
      (cond
        (== i hi) (do (aset a hi (aget a store))
                      (aset a store pivot)
                      store)
        (< (aget a i) pivot) (let [t (aget a i)]
                               (aset a i (aget a store))
                               (aset a store t)
                               (recur (inc i) (inc store)))
        :else (recur (inc i) store)))))

(defn- select
  "The k-th smallest of the first n values of a, by quickselect. Reorders those values. The
   same number a sort would put at index k, in linear time: measure needs only the median and
   one cutoff, not the whole order."
  ^double [^doubles a ^long n ^long k]
  (loop [lo 0 hi (dec n)]
    (if (>= lo hi)
      (aget a k)
      (let [p (split-at-pivot a lo hi)]
        (cond
          (== k p) (aget a k)
          (< k p) (recur lo (dec p))
          :else (recur (inc p) hi))))))

(defn- measure
  "Fills slot for one yaw and returns true. Slot is rmse, pitch, used, gsigma, hsigma, corr,
   loss. Pitch is the mean residual of the inliers: what is left of the tilt after the camera's
   own pitch. gsigma and hsigma are the relief of the
   photograph and the horizon. Loss is the trimmed error in degrees, plus the log gap between
   those reliefs once it exceeds one step of the field-of-view grid, plus twice the correlation
   miss. Dividing the error by the relief made a tall wrong ridge look as close as the real one.
   False when too few columns have a horizon, or the
   photograph's relief is under min-relief. Columns closer than near-m may be a foothill. Past
   near-keep of the window they are the slope under the camera, and that fraction adds
   near-weight degrees. A column closer than floor-m is the walk's first step. That fraction
   adds floor-weight degrees, with no allowance: a berm at the start of the walk is not a ridge.
   Hints sit in the let: a primitive function cannot take this many arrays."
  [angles distances base azimuths offsets geoms yaw gs hs ds diffs sorted devs slot]
  (let [^shorts angles angles
        ^floats distances distances
        base (long base)
        azimuths (long azimuths)
        ^doubles offsets offsets
        ^doubles geoms geoms
        yaw (long yaw)
        ^doubles gs gs
        ^doubles hs hs
        ^doubles ds ds
        ^doubles diffs diffs
        ^doubles sorted sorted
        ^doubles devs devs
        ^doubles slot slot
        _ (trace/bump :measure)
        n (alength offsets)]
    ;; A loop bound in a let is compiled as a boxed inner function. As a statement it stays a
    ;; primitive loop, and hands its count back through slot 2.
    (loop [i 0 k 0]
      (if (== i n)
        (aset slot 2 (double k))
        (let [az (wrap (+ (double yaw) (aget offsets i)))
              h (angle-at angles base azimuths az)]
          (if (Double/isNaN h)
            (recur (inc i) k)
            (let [g (aget geoms i)
                  hd (/ h 100.0)]
              (aset gs k g)
              (aset hs k hd)
              (aset ds k (double (aget distances (int (+ base (long az))))))
              (aset diffs k (- hd g))
              (recur (inc i) (inc k)))))))
    (let [used (long (aget slot 2))]
    (cond
      (< used (max 24 (long (* 0.45 n)))) (do (trace/bump :reject-columns) false)
      :else
      (let [keep (max 1 (int (* (double keep-fraction) used)))
            nused (int used)]
        (System/arraycopy diffs 0 sorted 0 nused)
        (let [median (select sorted nused (quot nused 2))]
          (dotimes [i nused]
            (aset devs i (Math/abs (- (aget diffs i) median))))
          (let [cutoff (select devs nused (dec keep))
                acc (double-array 10)]
            (dotimes [i nused]
              (let [r (aget diffs i)
                    a (Math/abs (- r median))]
                (when (<= a cutoff)
                  (let [g (aget gs i)
                        h (aget hs i)
                        d (aget ds i)]
                    (aset acc 0 (+ (aget acc 0) 1.0))
                    (aset acc 1 (+ (aget acc 1) r))
                    (aset acc 2 (+ (aget acc 2) (* r r)))
                    (aset acc 3 (+ (aget acc 3) g))
                    (aset acc 4 (+ (aget acc 4) h))
                    (aset acc 5 (+ (aget acc 5) (* g g)))
                    (aset acc 6 (+ (aget acc 6) (* h h)))
                    (aset acc 7 (+ (aget acc 7) (* g h)))
                    (when (and (> d 0.0) (< d (double near-m)))
                      (aset acc 8 (+ (aget acc 8) 1.0)))
                    (when (and (> d 0.0) (< d (double floor-m)))
                      (aset acc 9 (+ (aget acc 9) 1.0)))))))
            (let [k (aget acc 0)
                  pitch (/ (aget acc 1) k)
                  mse (- (/ (aget acc 2) k) (* pitch pitch))
                  rmse (Math/sqrt (max 0.0 mse))
                  mg (/ (aget acc 3) k)
                  mh (/ (aget acc 4) k)
                  vg (max 0.0 (- (/ (aget acc 5) k) (* mg mg)))
                  vh (max 0.0 (- (/ (aget acc 6) k) (* mh mh)))
                  gsigma (Math/sqrt vg)
                  hsigma (Math/sqrt vh)
                  denom (Math/sqrt (* vg vh))
                  corr (if (< denom 1.0e-6) 0.0
                           (max -1.0 (min 1.0 (/ (- (/ (aget acc 7) k) (* mg mh)) denom))))
                  floor-s (max gsigma 1.0e-3)
                  ;; One step of the coarse field of view moves the relief by about this much.
                  ;; A gap inside that is the lens grid, not a wrong distance.
                  scale (max 0.0 (- (Math/abs (Math/log (/ (max hsigma 1.0e-3) floor-s))) 0.10))
                  near (/ (aget acc 8) k)
                  ;; The allowance is a foothill in the frame. The rest of a valley photograph
                  ;; is the range, kilometres out. A hillside stand matches by looking at itself.
                  slope (max 0.0 (- near (double near-keep)))
                  floor (/ (aget acc 9) k)
                  loss (+ rmse
                          scale
                          (* 2.0 (- 1.0 corr))
                          (* (double near-weight) slope)
                          (* (double floor-weight) floor))]
              (cond
                (< gsigma (double min-relief)) (do (trace/bump :reject-relief) false)
                :else
                (do (aset slot 0 rmse)
                    (aset slot 1 pitch)
                    (aset slot 2 k)
                    (aset slot 3 gsigma)
                    (aset slot 4 hsigma)
                    (aset slot 5 corr)
                    (aset slot 6 loss)
                    true))))))))))

(defn- drop-penalty
  "Degrees added when the ground foreground-m ahead is below the camera. fall is that drop in
   meters. A rise, or a fall inside drop-allow-m, adds nothing."
  ^double [^double fall]
  (max 0.0 (/ (- fall (double drop-allow-m)) (double drop-scale-m))))

(defn- falls
  "Meters the ground foreground-m away is below the camera, one value per degree of azimuth.
   Positive when the earth drops. Off the grid is zero: missing ground is not a canyon."
  ^doubles [grid lat lon ground]
  (let [out (double-array horizon/azimuths)
        lat (double lat)
        lon (double lon)
        ground (double ground)
        lat-m 111320.0
        lon-m (* lat-m (Math/cos (Math/toRadians lat)))
        dist (double foreground-m)]
    (dotimes [az horizon/azimuths]
      (let [r (Math/toRadians (double az))
            z (dem/elev-at grid
                           (+ lat (/ (* dist (Math/cos r)) lat-m))
                           (+ lon (/ (* dist (Math/sin r)) lon-m)))]
        (aset out az (double (if (nil? z) 0.0 (- ground (double z)))))))
    out))

(defn- tilt-guess
  "Degrees the level camera must tilt for its skyline to sit on the horizon at yaw, or NaN when
   too few columns have a horizon. The mean gap between the horizon and the level skyline is
   the pitch seen through kappa."
  [angles base azimuths offsets geoms yaw kappa]
  (let [^shorts angles angles
        base (long base)
        azimuths (long azimuths)
        ^doubles offsets offsets
        ^doubles geoms geoms
        yaw (double (long yaw))
        kappa (double kappa)
        n (alength offsets)]
    (loop [i 0 k 0 sum 0.0]
      (if (== i n)
        (if (< k (max 24 (long (* 0.45 n))))
          Double/NaN
          (/ sum (* (double k) kappa)))
        (let [h (angle-at angles base azimuths (wrap (+ yaw (aget offsets i))))]
          (if (Double/isNaN h)
            (recur (inc i) k sum)
            (recur (inc i) (inc k) (+ sum (- (/ h 100.0) (aget geoms i))))))))))

(defn- tilt-at
  "The camera of tilts pitched nearest to degrees, or nil when degrees is NaN or outside the
   pitches a skyline photograph is taken at."
  [tilts degrees]
  (let [degrees (double degrees)]
    (cond
      (Double/isNaN degrees) nil
      :else
      (let [i (- (Math/round degrees) (long pitch-low))]
        (when (and (<= 0 i) (< i (count tilts)))
          (nth tilts i))))))

(defn- fit
  "The tilted camera of lens whose skyline sits on the horizon at yaw, with slot filled by
   measure, or nil. The level camera guesses the tilt. When the residual left over is more than
   pitch-slack the guess was bent by the wide frame, and the camera tilts once more by what was
   left."
  [angles distances base azimuths lens yaw gs hs ds diffs sorted devs slot]
  (trace/bump :fit)
  (let [^doubles slot slot
        tilts (:lens/tilts lens)
        level (:lens/level lens)
        kappa (double (:lens/kappa lens))
        slack (double pitch-slack)
        attempt (fn [cam]
                  (when-not cam (trace/bump :reject-pitch))
                  (when (and cam
                             (measure angles distances base azimuths
                                      (:camera/offsets cam) (:camera/geoms cam)
                                      yaw gs hs ds diffs sorted devs slot))
                    cam))
        one (attempt (tilt-at tilts
                              (tilt-guess angles base azimuths
                                          (:camera/offsets level) (:camera/geoms level)
                                          yaw kappa)))]
    (cond
      (nil? one) nil
      (<= (Math/abs (aget slot 1)) slack) one
      :else
      (let [_ (trace/bump :second-tilt)
            two (attempt (tilt-at tilts
                                  (+ (double (:camera/pitch one)) (/ (aget slot 1) kappa))))]
        (cond
          (nil? two) nil
          (<= (Math/abs (aget slot 1)) slack) two
          :else nil)))))

(defn- best-yaw
  "The yaw, pitch, relief and loss of one lens at one place, or nil. falls is the foreground
   drop per degree. Hints sit in the let: a primitive function cannot take the angles and the
   distances."
  [angles distances base azimuths lens falls yaws]
  (let [^ints yaws yaws
        m (alength yaws)
        ^shorts angles angles
        ^floats distances distances
        base (long base)
        azimuths (long azimuths)
        ^doubles falls falls
        ^doubles offsets (:camera/offsets (:lens/level lens))
        n (alength offsets)
        gs (double-array n)
        hs (double-array n)
        ds (double-array n)
        diffs (double-array n)
        sorted (double-array n)
        devs (double-array n)
        slot (double-array 7)]
    (loop [j 0 best ##Inf byaw 0.0 bpitch 0.0 bused 0.0
           brmse 0.0 bgs 0.0 bhs 0.0 bcorr 0.0 bcam nil]
      (if (== j m)
        (when bcam
          {:loss best :rmse brmse :yaw byaw :pitch bpitch :used bused
           :gsigma bgs :hsigma bhs :corr bcorr :cam bcam})
        (let [yaw (aget yaws j)
              cam (fit angles distances base azimuths lens yaw gs hs ds diffs sorted devs slot)]
          (if cam
            (let [lens-scale (- (/ (double ref-fov) (double (:camera/fov cam))) 1.0)
                  loss (+ (aget slot 6)
                          (* (aget slot 0) lens-scale)
                          (drop-penalty (aget falls yaw)))]
              (if (< loss best)
                (recur (inc j) loss (double yaw)
                       (+ (double (:camera/pitch cam)) (aget slot 1)) (aget slot 2)
                       (aget slot 0) (aget slot 3) (aget slot 4) (aget slot 5) cam)
                (recur (inc j) best byaw bpitch bused brmse bgs bhs bcorr bcam)))
            (recur (inc j) best byaw bpitch bused brmse bgs bhs bcorr bcam)))))))

(defn- at-place
  "The best yaw of every lens at one place. Each field of view keeps its own relief, and the
   loss compares them on that relief. grid supplies the foreground drop."
  [field grid cameras yaws i]
  (let [^shorts angles (:field/angles field)
        ^floats distances (:field/distances field)
        ^doubles lats (:field/lats field)
        ^doubles lons (:field/lons field)
        ^floats grounds (:field/grounds field)
        azimuths (long (:field/azimuths field))
        i (long i)
        base (* i azimuths)
        _ (trace/bump :places)
        falls (falls grid (aget lats i) (aget lons i) (double (aget grounds i)))]
    (into []
          (keep (fn [cam]
                  (when-let [hit (best-yaw angles distances base azimuths cam falls yaws)]
                    (assoc hit :i i))))
          cameras)))

(defn- place-of
  "The ranked map for a winning consideration."
  [field hit]
  (let [^doubles lats (:field/lats field)
        ^doubles lons (:field/lons field)
        ^floats grounds (:field/grounds field)
        i (long (:i hit))
        cam (:cam hit)]
    {:place/index i
     :place/lat (aget lats i)
     :place/lon (aget lons i)
     :place/elev (double (aget grounds i))
     :place/yaw (double (:yaw hit))
     :place/fov (:camera/fov cam)
     :place/roll (:camera/roll cam)
     :place/pitch (double (:pitch hit))
     :place/rmse (double (:rmse hit))
     :place/gsigma (double (:gsigma hit))
     :place/hsigma (double (:hsigma hit))
     :place/corr (double (:corr hit))
     :place/loss (double (:loss hit))
     :place/used (long (:used hit))}))

(defn- best-hit
  "The smallest loss among one place's cameras, or nil."
  [hits]
  (when (seq hits)
    (apply min-key :loss hits)))

(defn surface
  "The best camera at every place that scored, best first. width and height are the
   photograph's. grid is the elevation the foreground drop is read from. Each place is one item
   in a wave. opts narrows the search: :fovs and :rolls are the lenses tried (default the full
   set), :yaws-of is a function of place index to the int array of yaws tried (default every
   degree)."
  ([field grid skyline width height] (surface field grid skyline width height {}))
  ([field grid skyline width height {:keys [fovs rolls yaws-of]}]
   (let [cams (lenses width height skyline (or fovs *fovs*) (or rolls viewfinder.score/rolls))
         yaws-of (or yaws-of (constantly all-yaws))
         n (long (:field/count field))
         hits (into []
                    (keep best-hit)
                    (wave/across (fn [i] (at-place field grid cams (yaws-of i) i))
                                 (range n)))]
     (mapv (fn [hit] (place-of field hit)) (sort-by :loss hits)))))

(defn locate
  "The places in field whose horizon best matches the skyline, smallest loss first. width and
   height are the photograph's. grid is the elevation the foreground drop is read from.
   Each place is one item in a wave."
  [field grid skyline width height]
  (let [cams (lenses width height skyline)
        n (long (:field/count field))
        done (atom 0)
        hits (into []
                   (mapcat identity)
                   (wave/across (fn [i]
                                  (let [hit (at-place field grid cams all-yaws i)
                                        k (long (swap! done inc))]
                                    (when (== 0 (rem k 50))
                                      (println {:scored k :of n})
                                      (flush))
                                    hit))
                                (range n)))]
    (->> hits
         (sort-by :loss)
         (take top-k)
         (mapv (fn [hit] (place-of field hit))))))

(defn outline
  "The horizon of one place drawn through that place's camera: [x y] pixels of the photograph,
   one per degree of azimuth that lands inside the frame. seen is horizon/view of the place and
   place is a ranked map. This is ray run backwards: yaw, then pitch, then roll."
  [seen place width height]
  (let [^shorts angles (:horizon/angles seen)
        width (double width)
        height (double height)
        yaw (long (:place/yaw place))
        fov (double (:place/fov place))
        roll (math/to-radians (double (:place/roll place)))
        pitch (math/to-radians (double (:place/pitch place)))
        f (/ width 2.0 (math/tan (math/to-radians (/ fov 2.0))))
        cr (math/cos roll)
        sr (math/sin roll)
        cp (math/cos pitch)
        sp (math/sin pitch)
        reach (long (math/ceil (+ (/ fov 2.0) 10.0)))]
    (into []
          (keep (fn [step]
                  (let [step (long step)
                        c (long (aget angles (int (mod (+ yaw step) 360))))]
                    (when-not (== c (long horizon/missing))
                      (let [e (math/to-radians (/ (double c) 100.0))
                            a (math/to-radians (double step))
                            right (* (math/cos e) (math/sin a))
                            up (math/sin e)
                            forward (* (math/cos e) (math/cos a))
                            lifted (- (* up cp) (* forward sp))
                            depth (+ (* forward cp) (* up sp))]
                        (when (pos? depth)
                          (let [xr (/ (* f right) depth)
                                yr (/ (* f lifted) depth)
                                x (+ (/ width 2.0) (+ (* xr cr) (* yr sr)))
                                y (- (/ height 2.0) (- (* yr cr) (* xr sr)))]
                            (when (and (<= 0.0 x) (<= x width))
                              [x y]))))))))
          (range (- reach) (inc reach)))))

(defn- bracket
  "The index j with offsets[j] <= target < offsets[j+1], clamped to the last pair. offsets
   rises."
  ^long [^doubles offsets ^double target]
  (let [last-pair (- (alength offsets) 2)]
    (loop [j 0]
      (if (and (< j last-pair) (< (aget offsets (inc j)) target))
        (recur (inc j))
        j))))

(defn- template
  "The photograph's skyline seen through a camera of fov tilted pitch degrees, resampled to one
   elevation per whole degree of azimuth offset: {:fov :start :elevs} where elevs[k] is at
   offset start + k. Tilt matters: a wide frame tilted up bends its edge columns down, which a
   constant cannot absorb, so the screen tries a few tilts."
  [width height fov pitch skyline]
  (let [cam (camera width height fov 0.0 pitch skyline)
        ^doubles offsets (:camera/offsets cam)
        ^doubles geoms (:camera/geoms cam)
        n (alength offsets)
        start (long (Math/ceil (aget offsets 0)))
        end (long (Math/floor (aget offsets (dec n))))
        m (max 0 (inc (- end start)))
        elevs (double-array m)]
    (dotimes [k m]
      (let [target (double (+ start k))
            j (bracket offsets target)
            o0 (aget offsets j)
            o1 (aget offsets (inc j))
            t (if (> o1 o0) (/ (- target o0) (- o1 o0)) 0.0)]
        (aset elevs k (+ (aget geoms j) (* (max 0.0 (min 1.0 t)) (- (aget geoms (inc j)) (aget geoms j)))))))
    (let [acc (double-array 2)]
      ;; gk is computed once per template; areduce boxes but runs a few hundred times a search.
      (dotimes [k m]
        (aset acc 0 (+ (aget acc 0) (aget elevs k)))
        (aset acc 1 (+ (aget acc 1) (* (aget elevs k) (aget elevs k)))))
      {:fov (double fov) :start start :elevs elevs :g1 (aget acc 0) :g2 (aget acc 1)
       :gk (areduce elevs k sum 0.0 (+ sum (* (double k) (aget elevs k))))})))

(def screen-yaw-span
  "The screen scores every fov / this many degrees (at least one), then the degrees between
   around its best. A step that is a fixed share of the frame: a telephoto peak is a few
   degrees wide, an ultrawide skyline changes slowly."
  25)

(def screen-tilt
  "Largest skyline slope, degrees of elevation per degree of azimuth, the screen credits to roll.
   About three degrees of roll."
  0.055)

(defn- dot
  "Sum of g[k] * e[b + k] over g. Four running sums, so the processor overlaps the adds instead
   of waiting on each in turn; this loop is most of the screen's time. Its own function so the
   loop stays primitive."
  ^double [^doubles g ^doubles e ^long b]
  (let [m (alength g)
        quad (- m (rem m 4))]
    (loop [k 0 a0 0.0 a1 0.0 a2 0.0 a3 0.0]
      (if (< k quad)
        (let [j (+ b k)]
          (recur (+ k 4)
                 (+ a0 (* (aget g k) (aget e j)))
                 (+ a1 (* (aget g (+ k 1)) (aget e (+ j 1))))
                 (+ a2 (* (aget g (+ k 2)) (aget e (+ j 2))))
                 (+ a3 (* (aget g (+ k 3)) (aget e (+ j 3))))))
        (loop [k k acc (+ (+ a0 a1) (+ a2 a3))]
          (if (< k m)
            (recur (inc k) (+ acc (* (aget g k) (aget e (+ b k)))))
            acc))))))

(defn- slide-yaw
  "One yaw of slide: the window's sums from the running totals, its cross term, and the loss.
   Keeps it in out when it beats what out holds."
  [^doubles e ^doubles p0 ^doubles p2 ^doubles pj ^doubles pn ^doubles g ^doubles falls
   ^doubles out ^doubles q]
  ;; q holds the template's constants and the yaw, so nothing is boxed per call:
  ;; start g1 g2 gk k1 sxx mg vg gsigma lens-scale yaw
  (let [m (alength g)
        start (long (aget q 0))
        g1 (aget q 1)
        g2 (aget q 2)
        gk (aget q 3)
        k1 (aget q 4)
        sxx (aget q 5)
        mg (aget q 6)
        vg (aget q 7)
        gsigma (aget q 8)
        lens-scale (aget q 9)
        yaw (long (aget q 10))
        mf (double m)]
    (let [b (rem (+ yaw start 720) 360)
          end (+ b m)]
      (when (== 0.0 (- (aget pn end) (aget pn b)))
        (let [s1 (- (aget p0 end) (aget p0 b))
              s2 (- (aget p2 end) (aget p2 b))
              sk (- (- (aget pj end) (aget pj b)) (* (double b) s1))
              cross (dot g e b)
              sr (- s1 g1)
              srr (+ (- s2 (* 2.0 cross)) g2)
              skr (- sk gk)
              sxr (- skr (/ (* k1 sr) mf))
              ;; The tilt a small roll puts on the skyline, fitted to the residual and
              ;; capped, then applied to the photograph's skyline for every term.
              tilt (if (> sxx 0.0)
                     (max (- (double screen-tilt)) (min (double screen-tilt) (/ sxr sxx)))
                     0.0)
              rss (+ (- srr (/ (* sr sr) mf)) (* -2.0 tilt sxr) (* tilt tilt sxx))
              mse (max 0.0 (/ rss mf))
              mh (/ s1 mf)
              vh (max 0.0 (- (/ s2 mf) (* mh mh)))
              kbar (/ k1 mf)
              cgk (- (/ gk mf) (* mg kbar))
              chk (- (/ sk mf) (* mh kbar))
              vt (max 0.0 (+ vg (* tilt tilt (/ sxx mf)) (* 2.0 tilt cgk)))
              denom (Math/sqrt (* vt vh))
              corr (if (< denom 1.0e-9)
                     0.0
                     (/ (+ (- (/ cross mf) (* mh mg)) (* tilt chk)) denom))
              scale (max 0.0 (- (Math/abs (Math/log (/ (max (Math/sqrt vh) 1.0e-3)
                                                        (max (Math/sqrt vt) 1.0e-3))))
                                0.10))
              loss (+ (* (Math/sqrt mse) lens-scale)
                      scale
                      (* 2.0 (- 1.0 corr))
                      (drop-penalty (aget falls yaw)))]
          (when (< loss (aget out 0))
            (aset out 0 loss)
            (aset out 1 (double yaw))))))))

(defn- slide
  "The best yaw of one template against one place's horizon, every whole degree. The horizon
   is prepared once per place (see horizon-sums): twice round, with running sums of the
   value, its square, its index times the value, and missing degrees. Each window's sums are
   then two reads, and only the cross term needs a pass over the template. The residual is
   fitted with a straight line before its error is taken, which absorbs a small roll. Fills out
   with loss and yaw; false when no window has the whole template on ground. A screen: only
   the order it gives is used."
  [^doubles e ^doubles p0 ^doubles p2 ^doubles pj ^doubles pn tpl ^doubles falls ^doubles out]
  (let [^doubles g (:elevs tpl)
        m (alength g)
        start (long (:start tpl))
        lens-scale (/ (double ref-fov) (double (:fov tpl)))
        g1 (double (:g1 tpl))
        g2 (double (:g2 tpl))
        gk (double (:gk tpl))
        mf (double m)
        k1 (/ (* mf (- mf 1.0)) 2.0)
        k2 (/ (* (- mf 1.0) mf (- (* 2.0 mf) 1.0)) 6.0)
        sxx (- k2 (/ (* k1 k1) mf))
        mg (/ g1 mf)
        vg (max 0.0 (- (/ g2 mf) (* mg mg)))
        gsigma (Math/sqrt vg)]
    (aset out 0 Double/POSITIVE_INFINITY)
    (cond
      (or (< m 8) (< gsigma (double min-relief))) false
      :else
      (do
        (let [step (max 1 (long (quot (long (:fov tpl)) (long screen-yaw-span))))
              q (doto (double-array 11)
                  (aset 0 (double start)) (aset 1 g1) (aset 2 g2) (aset 3 gk) (aset 4 k1)
                  (aset 5 sxx) (aset 6 mg) (aset 7 vg) (aset 8 gsigma) (aset 9 lens-scale))]
          (dotimes [j (quot 360 step)]
            (aset q 10 (double (* j step)))
            (slide-yaw e p0 p2 pj pn g falls out q))
          (when (Double/isFinite (aget out 0))
            (let [center (long (aget out 1))]
              (loop [d 1]
                (when (< d step)
                  (aset q 10 (double (rem (+ center d) 360)))
                  (slide-yaw e p0 p2 pj pn g falls out q)
                  (aset q 10 (double (rem (+ center (- 360 d)) 360)))
                  (slide-yaw e p0 p2 pj pn g falls out q)
                  (recur (inc d)))))))
        (Double/isFinite (aget out 0))))))

(defn- horizon-sums
  "One place's horizon in degrees, twice round so a window never wraps, with running sums:
   e, then prefix sums of e, e squared, index times e, and missing degrees (as 1.0). A missing
   degree is 0 in the value sums; the missing count rules its windows out."
  [^shorts angles base]
  (let [base (long base)
        e (double-array 720)
        p0 (double-array 721)
        p2 (double-array 721)
        pj (double-array 721)
        pn (double-array 721)]
    (dotimes [j 720]
      (let [c (long (aget angles (int (+ base (rem j 360)))))
            gone (== c (long horizon/missing))
            v (if gone 0.0 (/ (double c) 100.0))]
        (aset e j v)
        (aset p0 (inc j) (+ (aget p0 j) v))
        (aset p2 (inc j) (+ (aget p2 j) (* v v)))
        (aset pj (inc j) (+ (aget pj j) (* (double j) v)))
        (aset pn (inc j) (+ (aget pn j) (if gone 1.0 0.0)))))
    [e p0 p2 pj pn]))

(defn- slide-place
  "The best screen loss, yaw and lens of one place over every template."
  [field grid templates i]
  (let [^shorts angles (:field/angles field)
        ^doubles lats (:field/lats field)
        ^doubles lons (:field/lons field)
        ^floats grounds (:field/grounds field)
        azimuths (long (:field/azimuths field))
        i (long i)
        [e p0 p2 pj pn] (horizon-sums angles (* i azimuths))
        falls (falls grid (aget lats i) (aget lons i) (double (aget grounds i)))
        out (double-array 2)]
    (trace/bump :places)
    (reduce (fn [best tpl]
              (if (and (slide e p0 p2 pj pn tpl falls out)
                       (or (nil? best) (< (aget out 0) (double (:place/loss best)))))
                {:place/index i
                 :place/lat (aget lats i)
                 :place/lon (aget lons i)
                 :place/loss (aget out 0)
                 :place/yaw (aget out 1)
                 :place/fov (:fov tpl)}
                best))
            nil
            templates)))

(def screen-pitches
  "Tilts the screen tries, degrees. Six apart: the full search refines from the nearest."
  [-3.0 3.0 9.0 15.0 21.0])

(defn slide-surface
  "The screen by sliding sums over every place of field, best first. One template per lens and
   tilt: every other lens of *fovs* (all of them when the lens is known) at screen-pitches."
  [field grid skyline width height]
  (let [fovs *fovs*
        fovs (if (<= (count fovs) 3) fovs (into [] (take-nth 2) fovs))
        templates (vec (for [fov fovs
                             pitch screen-pitches]
                         (template width height fov pitch skyline)))
        n (long (:field/count field))]
    (->> (wave/across (fn [i] (slide-place field grid templates i)) (range n))
         (keep identity)
         (sort-by :place/loss)
         vec)))

(defn rank
  "The best camera at the most likely places of field, best first, in two passes. The screen
   scores every place cheaply: by default slide-surface, every yaw at once by sliding sums
   (-Dviewfinder.screen=sparse instead uses measure on a quarter of the columns, a third of the
   lenses, no roll, every screen-step degrees). The best keep places then get the full search, with yaw
   only within reach degrees of where the screen pointed them. opts may set :keep, :reach,
   :screen-step and :screen-every (which columns and lenses the screen keeps)."
  ([field grid skyline width height] (rank field grid skyline width height {}))
  ([field grid skyline width height {:keys [keep reach screen-step screen-every]}]
   (let [n (long (:field/count field))
         keep (long (or keep (max 200 (long (* 0.03 n)))))
         reach (long (or reach 8))
         screen-step (long (or screen-step 2))
         every (long (or screen-every 4))
         screen (if (= "sparse" (System/getProperty "viewfinder.screen"))
                  (let [sparse (into [] (take-nth every) skyline)
                        fovs *fovs*
                        screen-fovs (if (<= (count fovs) 3) fovs (into [] (take-nth 3) fovs))]
                    (surface field grid sparse width height
                             {:fovs screen-fovs :rolls [0.0]
                              :yaws-of (constantly (yaws-every screen-step))}))
                  (slide-surface field grid skyline width height))
         chosen (vec (take keep screen))
         part (field/subset field (map :place/index chosen))
         centers (mapv :place/yaw chosen)]
     (with-meta
       (surface part grid skyline width height
                {:yaws-of (fn [i] (yaws-around (nth centers i) reach))})
       {:screen screen}))))
