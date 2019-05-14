(
    rpm: 1280.0376,
    intake_volume: 0.57453525,
    exhaust_volume: 0.26398277,
    engine_vibrations_volume: 0.16148204,
    cylinders: [
        (
            crank_offset: 0,
            exhaust_waveguide: (
                chamber0: (
                    samples: (delay:0.0009583333,),
                ),
                chamber1: (
                    samples: (delay:0.0009583333,),
                ),
                alpha: 0.8703525,
                beta: 0.06,
            ),
            intake_waveguide: (
                chamber0: (
                    samples: (delay:0.00014583333,),
                ),
                chamber1: (
                    samples: (delay:0.00014583333,),
                ),
                alpha: 1,
                beta: -0.49987197,
            ),
            extractor_waveguide: (
                chamber0: (
                    samples: (delay:0.0005833333,),
                ),
                chamber1: (
                    samples: (delay:0.0005833333,),
                ),
                alpha: 0,
                beta: 0.025332212,
            ),
            intake_open_refl: 0,
            intake_closed_refl: 1,
            exhaust_open_refl: 0,
            exhaust_closed_refl: 1,
            piston_motion_factor: 2.907945,
            ignition_factor: 4.305159,
            ignition_time: 0.07522921,
        ),// [0]
        (
            crank_offset: 0.23926818,
            exhaust_waveguide: (
                chamber0: (
                    samples: (delay:0.00033333333,),
                ),
                chamber1: (
                    samples: (delay:0.00033333333,),
                ),
                alpha: 0.9951538,
                beta: 0.06,
            ),
            intake_waveguide: (
                chamber0: (
                    samples: (delay:0.000125,),
                ),
                chamber1: (
                    samples: (delay:0.000125,),
                ),
                alpha: 1,
                beta: -0.49987197,
            ),
            extractor_waveguide: (
                chamber0: (
                    samples: (delay:0.00175,),
                ),
                chamber1: (
                    samples: (delay:0.00175,),
                ),
                alpha: 0,
                beta: 0.025332212,
            ),
            intake_open_refl: 0,
            intake_closed_refl: 1,
            exhaust_open_refl: 0,
            exhaust_closed_refl: 1,
            piston_motion_factor: 2.907945,
            ignition_factor: 4.305159,
            ignition_time: 0.07522921,
        ),// [1]
        (
            crank_offset: 0.47853637,
            exhaust_waveguide: (
                chamber0: (
                    samples: (delay:0.00070833333,),
                ),
                chamber1: (
                    samples: (delay:0.00070833333,),
                ),
                alpha: 1,
                beta: 0.06,
            ),
            intake_waveguide: (
                chamber0: (
                    samples: (delay:0.00022916666,),
                ),
                chamber1: (
                    samples: (delay:0.00022916666,),
                ),
                alpha: 0.38877177,
                beta: -0.49987197,
            ),
            extractor_waveguide: (
                chamber0: (
                    samples: (delay:0.0027083333,),
                ),
                chamber1: (
                    samples: (delay:0.0027083333,),
                ),
                alpha: 0,
                beta: 0.025332212,
            ),
            intake_open_refl: 0,
            intake_closed_refl: 1,
            exhaust_open_refl: 0,
            exhaust_closed_refl: 1,
            piston_motion_factor: 2.907945,
            ignition_factor: 4.305159,
            ignition_time: 0.07522921,
        ),// [2]
        (
            crank_offset: 0.71780455,
            exhaust_waveguide: (
                chamber0: (
                    samples: (delay:0.00045833332,),
                ),
                chamber1: (
                    samples: (delay:0.00045833332,),
                ),
                alpha: 1,
                beta: 0.06,
            ),
            intake_waveguide: (
                chamber0: (
                    samples: (delay:0.00039583334,),
                ),
                chamber1: (
                    samples: (delay:0.00039583334,),
                ),
                alpha: 1,
                beta: -0.49987197,
            ),
            extractor_waveguide: (
                chamber0: (
                    samples: (delay:0.004625,),
                ),
                chamber1: (
                    samples: (delay:0.004625,),
                ),
                alpha: 0,
                beta: 0.025332212,
            ),
            intake_open_refl: 0,
            intake_closed_refl: 1,
            exhaust_open_refl: 0,
            exhaust_closed_refl: 1,
            piston_motion_factor: 2.907945,
            ignition_factor: 4.305159,
            ignition_time: 0.07522921,
        ),// [3]
    ],
    intake_noise_factor: 0.27695146,
    intake_noise_lp: (
        samples: (
            delay: 0.00020833334,
        ),
        len: 9.744197,
    ),
    engine_vibration_filter: (
        samples: (
            delay: 0.0033333334,
        ),
        len: 160,
    ),
    muffler: (
        straight_pipe: (
            chamber0: (
                samples: (
                    delay: 0.00825,
                ),
            ),
            chamber1: (
                samples: (
                    delay: 0.00825,
                ),
            ),
            alpha: 0.17348766,
            beta: 0.024529576,
        ),
        muffler_elements: [
            (
                chamber0: (
                    samples: (delay:0.00091666664,),
                ),
                chamber1: (
                    samples: (delay:0.00091666664,),
                ),
                alpha: 0,
                beta: 0.1036278,
            ),// [0]
            (
                chamber0: (
                    samples: (delay:0.0010833333,),
                ),
                chamber1: (
                    samples: (delay:0.0010833333,),
                ),
                alpha: 0,
                beta: 0.1036278,
            ),// [1]
            (
                chamber0: (
                    samples: (delay:0.00125,),
                ),
                chamber1: (
                    samples: (delay:0.00125,),
                ),
                alpha: 0,
                beta: 0.1036278,
            ),// [2]
            (
                chamber0: (
                    samples: (delay:0.0013333333,),
                ),
                chamber1: (
                    samples: (delay:0.0013333333,),
                ),
                alpha: 0,
                beta: 0.1036278,
            ),// [3]
        ],
    ),
    intake_valve_shift: -0.03479871,
    exhaust_valve_shift: 0.006414771,
    crankshaft_fluctuation: 0.07406286,
    crankshaft_fluctuation_lp: (
        samples: (
            delay: 0.008854167,
        ),
        len: 425,
    ),
)